//! Command line client for t-rex. Talks to the server's REST API.

/// The command line: arguments, commands and their help text.
mod cli;
/// HTTP client for the REST API.
mod client;
/// Runs a parsed command against the server and prints the result.
mod commands;
/// Shell completion of project and absence names.
mod complete;
/// The saved server address and token.
mod config;
/// Browser sign-in: a local listener receives the one-time code.
mod login;
/// Plain-text output: aligned tables.
mod output;
/// Output templates for `entry list`: the table and the `--format` templates.
mod template;

use std::process::ExitCode;

use clap::{CommandFactory, Parser};

/// Entry point: answers shell completion requests, parses the arguments and runs the command. An error is printed to stderr and gives exit code 1.
fn main() -> ExitCode {
    // Answers shell completion requests (COMPLETE=bash|zsh|... t-rex) and exits when one is made.
    clap_complete::CompleteEnv::with_factory(cli::Cli::command).complete();
    let cli = cli::Cli::parse();
    match commands::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("t-rex: {err:#}");
            ExitCode::FAILURE
        }
    }
}
