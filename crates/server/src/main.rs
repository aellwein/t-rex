//! Entry point of the server. It reads the configuration, connects to the database, runs the migrations and
//! serves the REST API and the web app on port 8080.
//!
//! The OIDC settings `ISSUER`, `CLIENT_ID`, `CLIENT_SECRET` and `COOKIE_SECRET` are required. See
//! `docs/configuration.md` for all environment variables.

#![forbid(unsafe_code)]
use std::{env, sync::Arc};

use anyhow::Result;
#[cfg(feature = "embed")]
use axum::routing::get;
use axum_oidc_client::{
    auth::AuthenticationLayer,
    auth_builder::OAuthConfigurationBuilder,
    auth_cache::AuthCache,
    cache::{TwoTierAuthCache, config::TwoTierCacheConfig},
    logout::handle_default_logout::DefaultLogoutHandler,
};
use log::info;
use tokio::net::TcpListener;

#[cfg(feature = "embed")]
use embed::static_handler;

/// The REST API, the sign-in routes and the checks of who a request acts for.
mod api;
/// Settings read from the environment: the database and the server options.
mod config;
/// Connecting to SQLite or Postgres, and running the migrations.
mod db;
#[cfg(feature = "embed")]
/// The web app, built into the binary. Only with the `embed` feature.
mod embed;
/// The database tables as SeaORM models.
mod entity;
/// Schema migrations. They run on every start.
mod migration;
/// The queries against the database, one module per table.
mod store;

/// How long a browser sign-in lasts without activity, in minutes. The session cookie and the
/// server-side session both use this, so they expire together.
const SESSION_MINUTES: i64 = 12 * 60;

/// Main entry point. Setup and run the server.
#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // configure logging
    pretty_env_logger::init();

    let issuer = env::var("ISSUER").expect("ISSUER env variable");
    let client_id = env::var("CLIENT_ID").expect("CLIENT_ID env variable");
    let client_secret = env::var("CLIENT_SECRET").expect("needs CLIENT_SECRET");
    let app_url = env::var("APP_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let cookie_secret = env::var("COOKIE_SECRET").expect("COOKIE_SECRET env variable");

    let config = OAuthConfigurationBuilder::default()
        .with_issuer(issuer.as_str())
        .await?
        .with_client_id(client_id.as_str())
        .with_client_secret(client_secret.as_str())
        .with_redirect_uri(format!("{app_url}/auth/callback").as_str())
        .with_private_cookie_key(cookie_secret.as_str())
        .with_session_max_age(SESSION_MINUTES)
        .with_scopes(vec!["openid", "profile", "email"])
        .with_post_logout_redirect_uri("/")
        .build()?;

    let cache: Arc<dyn AuthCache + Send + Sync> = Arc::new(TwoTierAuthCache::new(
        None,
        TwoTierCacheConfig {
            // The in-memory session lives as long as the cookie. The library reads this value
            // in seconds and resets it on every request, so the session is idle-based like the cookie.
            l1_ttl_sec: (SESSION_MINUTES * 60) as u64,
            ..TwoTierCacheConfig::default()
        },
    )?);

    let logout_handler = Arc::new(DefaultLogoutHandler);

    // database: created on first start, migrated on every start
    let database = config::database_config().map_err(anyhow::Error::msg)?;
    let conn = db::connect_config(&database).await?;
    db::migrate(&conn).await?;
    info!("database ready at {}", database.describe());
    let state = api::AppState::new(conn);

    let api = api::router(state);
    #[cfg(feature = "embed")]
    let api = api.fallback(get(static_handler));
    let app = api.layer(AuthenticationLayer::new(
        Arc::new(config),
        cache,
        logout_handler,
    ));

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;

    // Tag and commit from git at build time (for example `v1.2-3-gabc1234`), or the commit alone.
    let version = option_env!("VERGEN_GIT_DESCRIBE")
        .or(option_env!("VERGEN_GIT_SHA"))
        .unwrap_or("unknown");
    info!("t-rex running at commit {version} on {app_url}");
    info!("listening on {addr}");
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
