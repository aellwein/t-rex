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

mod api;
mod config;
mod db;
#[cfg(feature = "embed")]
mod embed;
mod entity;
mod migration;
mod store;

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
        .with_session_max_age(30)
        .with_scopes(vec!["openid", "profile", "email"])
        .with_post_logout_redirect_uri("/")
        .build()?;

    let cache: Arc<dyn AuthCache + Send + Sync> =
        Arc::new(TwoTierAuthCache::new(None, TwoTierCacheConfig::default())?);

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
