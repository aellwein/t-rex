//! REST API under `/api/v1`. Every handler acts for the current user only.

pub mod absences;
pub mod cli;
pub mod entries;
pub mod error;
pub mod holidays;
pub mod identity;
pub mod me;
pub mod pending;
pub mod projects;
pub mod settings;
pub mod tokens;

#[cfg(test)]
mod tests;

#[cfg(test)]
use std::sync::Arc;

use axum::{
    Json, Router,
    routing::{get, post},
};
use dto::PageQuery;
pub use error::{ApiError, ApiResult};
pub use identity::{CurrentAccount, CurrentUser, SessionAccount};
use sea_orm::DatabaseConnection;

use crate::store::Window;

/// The window of a list that a request asks for. Bad `limit` values are a bad request.
pub(crate) fn window(query: &PageQuery) -> ApiResult<Window> {
    let (limit, offset) = query.window().map_err(ApiError::BadRequest)?;
    Ok(Window { limit, offset })
}

/// State shared by all handlers.
#[derive(Clone)]
pub struct AppState {
    conn: DatabaseConnection,
    /// Tests act as this user without an ID token.
    #[cfg(test)]
    test_user: Option<Arc<str>>,
}

impl AppState {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self {
            conn,
            #[cfg(test)]
            test_user: None,
        }
    }

    #[cfg(test)]
    pub async fn in_memory(user_id: &str) -> Self {
        let mut state = Self::new(crate::store::test_db().await);
        state.test_user = Some(Arc::from(user_id));
        state
    }
}

async fn health() -> &'static str {
    "ok"
}

/// The build the server runs. Taken from git at build time, see `build.rs`.
async fn version() -> Json<dto::VersionInfo> {
    let version = option_env!("VERGEN_GIT_DESCRIBE")
        .or(option_env!("VERGEN_GIT_SHA"))
        .unwrap_or("unknown");
    Json(dto::VersionInfo {
        version: version.to_string(),
    })
}

/// Builds the `/api/v1` routes. Authentication is applied by the caller around this router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/version", get(version))
        .route(
            "/api/v1/pending-start",
            get(pending::get).put(pending::put).delete(pending::delete),
        )
        .route(
            "/api/v1/projects",
            get(projects::list).post(projects::create),
        )
        .route(
            "/api/v1/projects/{id}",
            get(projects::get_one)
                .put(projects::update)
                .delete(projects::delete),
        )
        .route(
            "/api/v1/absence-kinds",
            get(absences::list).post(absences::create),
        )
        .route(
            "/api/v1/absence-kinds/{id}",
            get(absences::get_one)
                .put(absences::update)
                .delete(absences::delete),
        )
        .route(
            "/api/v1/time-entries",
            get(entries::list).post(entries::create),
        )
        .route(
            "/api/v1/time-entries/{id}",
            get(entries::get_one)
                .put(entries::update)
                .delete(entries::delete),
        )
        .route(
            "/api/v1/holidays",
            get(holidays::list).post(holidays::create),
        )
        .route("/api/v1/holidays/import", post(holidays::import))
        .route("/api/v1/holidays/import-text", post(holidays::import_text))
        .route(
            "/api/v1/holidays/{id}",
            get(holidays::get_one)
                .put(holidays::update)
                .delete(holidays::delete),
        )
        .route("/api/v1/me", get(me::get))
        .route("/api/v1/tokens", get(tokens::list).post(tokens::create))
        .route("/api/v1/tokens/{id}", axum::routing::delete(tokens::delete))
        .route("/api/v1/cli/login", get(cli::login))
        .route("/api/v1/cli/token", post(cli::exchange))
        .route("/api/v1/settings", get(settings::get).put(settings::update))
        .with_state(state)
}
