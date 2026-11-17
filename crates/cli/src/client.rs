//! HTTP client for the t-rex REST API. Every request carries the saved token, if there is one.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use dto::{ErrorBody, MAX_LIMIT, Page};
use reqwest::{
    Method, StatusCode,
    blocking::{Client as Http, RequestBuilder, Response},
};
use serde::{Serialize, de::DeserializeOwned};

/// HTTP client for the REST API. It holds the server address and the token, and turns the answers into values or readable errors.
pub struct Client {
    base: String,
    token: Option<String>,
    http: Http,
}

impl Client {
    /// A client for `server` with a 30 second timeout. The token, if any, is sent with every request.
    pub fn new(server: &str, token: Option<String>) -> Result<Self> {
        Self::with_timeout(server, token, Duration::from_secs(30))
    }

    /// A client with a short timeout, for shell completion: a slow server must not stall the shell.
    pub fn quick(server: &str, token: Option<String>) -> Result<Self> {
        Self::with_timeout(server, token, Duration::from_secs(2))
    }

    /// Builds a client with the given timeout. Fails if the server address is empty, and says how to set one.
    fn with_timeout(server: &str, token: Option<String>, timeout: Duration) -> Result<Self> {
        let base = server.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            bail!("no server address: run `t-rex login --server <URL>` or set T_REX_SERVER");
        }
        let http = Http::builder()
            .timeout(timeout)
            .user_agent(concat!("t-rex-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("creating the HTTP client")?;
        Ok(Self { base, token, http })
    }

    /// Starts a request to `path` on the server, with the bearer token when there is one.
    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let mut request = self.http.request(method, format!("{}{}", self.base, path));
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        request
    }

    /// Sends a GET request and decodes the JSON answer.
    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        read_json(
            self.request(Method::GET, path)
                .send()
                .context("could not reach the server")?,
        )
    }

    /// Reads every item of a list, one page at a time. `path` may already carry a query string.
    pub fn get_all<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>> {
        let separator = if path.contains('?') { '&' } else { '?' };
        let mut items = Vec::new();
        loop {
            let page: Page<T> = self.get(&format!(
                "{path}{separator}limit={MAX_LIMIT}&offset={}",
                items.len()
            ))?;
            let more = page.has_more() && !page.items.is_empty();
            items.extend(page.items);
            if !more {
                return Ok(items);
            }
        }
    }

    /// Sends `body` as JSON with the given method, and decodes the JSON answer.
    pub fn send<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let response = self
            .request(method, path)
            .json(body)
            .send()
            .context("could not reach the server")?;
        read_json(response)
    }

    /// Sends a DELETE request. Fails unless the server answers with a success status.
    pub fn delete(&self, path: &str) -> Result<()> {
        let response = self
            .request(Method::DELETE, path)
            .send()
            .context("could not reach the server")?;
        check(response).map(|_| ())
    }
}

/// Unauthenticated POST, used by the login exchange.
pub fn post_anonymous<B: Serialize, T: DeserializeOwned>(
    server: &str,
    path: &str,
    body: &B,
) -> Result<T> {
    let http = Http::builder().timeout(Duration::from_secs(30)).build()?;
    let base = server.trim().trim_end_matches('/');
    let response = http
        .post(format!("{base}{path}"))
        .json(body)
        .send()
        .context("could not reach the server")?;
    read_json(response)
}

/// Reads a successful answer as JSON, or turns the error answer into an error.
fn read_json<T: DeserializeOwned>(response: Response) -> Result<T> {
    let response = check(response)?;
    response
        .json()
        .context("the server sent an unexpected answer")
}

/// Turns error answers into messages a person can act on.
fn check(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if status == StatusCode::UNAUTHORIZED {
        bail!("not signed in: run `t-rex login`");
    }
    match response.json::<ErrorBody>() {
        Ok(body) => bail!("{}", body.error),
        Err(_) => bail!("the server answered {status}"),
    }
}
