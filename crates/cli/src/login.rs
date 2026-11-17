//! Browser sign-in. The CLI listens on a random port of 127.0.0.1, opens the server's sign-in page,
//! and receives a one-time code on that port. The code is then exchanged for a token.

use std::thread;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use dto::{CliTokenRequest, CreatedToken};

use crate::client::post_anonymous;

/// How long the browser has to finish the sign-in.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// Runs the browser sign-in against `server` and returns the new token.
pub fn sign_in(server: &str, device: &str) -> Result<CreatedToken> {
    let base = server.trim().trim_end_matches('/');
    let listener =
        TcpListener::bind("127.0.0.1:0").context("could not open a local port for the login")?;
    let port = listener.local_addr()?.port();
    let state = random_state();

    let mut login_url = reqwest::Url::parse(&format!("{base}/auth"))
        .context("the server address is not a valid URL")?;
    login_url.query_pairs_mut().append_pair(
        "redirect",
        &format!("/api/v1/cli/login?port={port}&state={state}"),
    );

    println!("Opening the browser to sign in...");
    println!("If it does not open, visit:\n  {login_url}");
    if open::that(login_url.as_str()).is_err() {
        println!("(could not start the browser)");
    }

    let code = wait_for_code(&listener, &state)?;
    post_anonymous(
        base,
        "/api/v1/cli/token",
        &CliTokenRequest {
            code,
            device: device.to_string(),
        },
    )
}

/// A random value for the login. The browser must send it back unchanged, which protects the login from forged callbacks.
fn random_state() -> String {
    let mut bytes = [0u8; 16];
    rand::fill(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Waits for the browser to call back with the code. Answers the browser with a short page.
fn wait_for_code(listener: &TcpListener, state: &str) -> Result<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false)?;
                match read_callback(&mut stream, state) {
                    Ok(code) => {
                        respond(
                            &mut stream,
                            "Signed in. You can close this tab and return to the terminal.",
                        );
                        return Ok(code);
                    }
                    Err(err) => {
                        // Stray requests (favicon, a repeated visit) are ignored until the deadline.
                        respond(
                            &mut stream,
                            "This sign-in link is not valid. Run `t-rex login` again.",
                        );
                        let _ = err; // stray request: nothing to do
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(err) => return Err(err).context("waiting for the browser"),
        }
        if Instant::now() >= deadline {
            bail!("the browser did not finish the sign-in in time; run `t-rex login` again");
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Reads the request line and returns the code if the state matches.
fn read_callback(stream: &mut TcpStream, state: &str) -> Result<String> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buffer = [0u8; 4096];
    let read = stream
        .read(&mut buffer)
        .context("the browser sent nothing")?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .context("the request has no path")?;
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .context("the path is not valid")?;
    if url.path() != "/callback" {
        bail!("unexpected path {}", url.path());
    }
    let mut code = None;
    let mut got_state = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => got_state = Some(value.into_owned()),
            _ => {}
        }
    }
    if got_state.as_deref() != Some(state) {
        bail!("the state does not match");
    }
    code.context("the callback has no code")
}

/// Answers the browser on the local listener with a short HTML page that shows `message`.
fn respond(stream: &mut TcpStream, message: &str) {
    let body =
        format!("<!doctype html><meta charset=\"utf-8\"><title>t-rex</title><p>{message}</p>");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}
