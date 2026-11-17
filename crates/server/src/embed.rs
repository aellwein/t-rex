//! Serving the web app from the binary: the built assets, the redirect to sign-in and the index page
//! for the app's own routes.

use axum::{
    http::{StatusCode, Uri, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_oidc_client::extractors::OptionalIdToken;
use rust_embed::RustEmbed;

/// Filename of the index page
static INDEX_HTML: &str = "index.html";

/// Built by `build.rs` (see `WEBAPP_PUBLIC_DIR` there) before the server is compiled.
#[derive(RustEmbed)]
#[folder = "../../target/webapp-build/dx/webapp/release/web/public/"]
struct Asset;

/// Serves the web app. A request without a sign-in is redirected to `/auth`. Existing files are served with their MIME type. Other paths without a dot return the index page, so the app's own routes work after a reload. A missing file with an extension answers `404`.
pub async fn static_handler(uri: Uri, id_token: OptionalIdToken) -> impl IntoResponse {
    if id_token.is_none() {
        return Redirect::temporary("/auth").into_response();
    }
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        return index_html().await;
    }
    if path == INDEX_HTML {
        return index_html().await;
    }
    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return not_found().await;
            }
            index_html().await
        }
    }
}

/// The index page of the web app, or `404` if the build has none.
async fn index_html() -> Response {
    match Asset::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found().await,
    }
}

/// A plain `404 Not Found` for files that do not exist.
async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404").into_response()
}
