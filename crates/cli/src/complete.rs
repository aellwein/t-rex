//! Shell completion for names that live on the server: projects and absence kinds. The shell asks for
//! candidates on each TAB, so this stays quick and quiet. Any problem gives no candidates.

use std::{env, ffi::OsStr};

use anyhow::Result;
use clap_complete::engine::CompletionCandidate;
use dto::{AbsenceKind, Project};

use crate::client::Client;
use crate::config;

/// Names of the active projects that start with what was typed.
pub fn complete_project(current: &OsStr) -> Vec<CompletionCandidate> {
    let names = fetch(|client| {
        let projects: Vec<Project> = client.get_all("/api/v1/projects")?;
        Ok(projects
            .into_iter()
            .filter(|project| !project.archived)
            .map(|project| project.name)
            .collect())
    });
    matching(names, current)
}

/// Names of the absence kinds that start with what was typed.
pub fn complete_absence(current: &OsStr) -> Vec<CompletionCandidate> {
    let names = fetch(|client| {
        let kinds: Vec<AbsenceKind> = client.get_all("/api/v1/absence-kinds")?;
        Ok(kinds.into_iter().map(|kind| kind.name).collect())
    });
    matching(names, current)
}

/// Runs one request with the saved server and token. Any failure gives no names.
fn fetch(request: impl FnOnce(&Client) -> Result<Vec<String>>) -> Vec<String> {
    let Ok(saved) = config::load() else {
        return Vec::new();
    };
    let server = env::var("T_REX_SERVER").ok().or(saved.server);
    let (Some(server), Some(token)) = (server, saved.token) else {
        return Vec::new();
    };
    let Ok(client) = Client::quick(&server, Some(token)) else {
        return Vec::new();
    };
    request(&client).unwrap_or_default()
}

/// Keeps the names that start with what is typed so far, ignoring case.
fn matching(names: Vec<String>, current: &OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy().to_lowercase();
    names
        .into_iter()
        .filter(|name| name.to_lowercase().starts_with(&prefix))
        .map(CompletionCandidate::new)
        .collect()
}
