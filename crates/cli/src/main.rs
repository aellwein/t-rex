//! Command line client for t-rex. Talks to the server's REST API.

mod cli;
mod client;
mod commands;
mod complete;
mod config;
mod login;
mod output;
mod template;

use std::process::ExitCode;

use clap::{CommandFactory, Parser};

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
