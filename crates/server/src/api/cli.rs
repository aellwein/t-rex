//! Browser login for the command line client.
//!
//! The client opens `/auth?redirect=/api/v1/cli/login?port=P&state=S` in the browser. After the normal
//! sign-in, this endpoint sends the browser to `http://127.0.0.1:P/callback?code=C&state=S`, where the
//! client listens. The client then trades the one-time code for a token at `POST /api/v1/cli/token`.
//! Codes are stored in the database, as hashes, for a few minutes and can be used once.

use std::time::Duration;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
};
use base64::Engine;
use dto::{CliTokenRequest, CreatedToken};
use serde::Deserialize;

use super::identity::{Account, hash_token};
use super::tokens::issue;
use super::{ApiError, ApiResult, AppState, SessionAccount};
use crate::store::api_tokens::TokenOwner;
use crate::store::cli_logins;

/// How long a login code stays valid.
const CODE_LIFETIME: Duration = Duration::from_secs(120);

#[derive(Debug, Deserialize)]
/// The query of the browser login: the local port of the command line client and the `state` value it sent. The state is sent back unchanged, so the client can check the answer.
pub struct LoginQuery {
    port: u16,
    state: String,
}

/// Sends a signed-in browser back to the client's local listener with a one-time code.
pub async fn login(
    State(state): State<AppState>,
    SessionAccount(account): SessionAccount,
    Query(query): Query<LoginQuery>,
) -> ApiResult<Redirect> {
    if query.port < 1024 {
        return Err(ApiError::BadRequest(
            "the port must be 1024 or higher".into(),
        ));
    }
    let state_ok = (16..=128).contains(&query.state.len())
        && query
            .state
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !state_ok {
        return Err(ApiError::BadRequest("invalid state parameter".into()));
    }

    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let code = base64_url(&bytes);
    let now = chrono::Utc::now().timestamp();
    let owner = TokenOwner {
        user_id: account.handle,
        email: account.email,
        display_name: account.name,
    };
    let expires_at = now + CODE_LIFETIME.as_secs() as i64;
    cli_logins::create(&state.conn, &hash_token(&code), &owner, expires_at, now).await?;

    let target = format!(
        "http://127.0.0.1:{}/callback?code={code}&state={}",
        query.port, query.state
    );
    Ok(Redirect::to(&target))
}

/// Exchanges a one-time login code for a token. The code is used up even if the exchange fails.
pub async fn exchange(
    State(state): State<AppState>,
    Json(input): Json<CliTokenRequest>,
) -> ApiResult<(StatusCode, Json<CreatedToken>)> {
    let now = chrono::Utc::now().timestamp();
    let owner = cli_logins::take(&state.conn, &hash_token(&input.code), now)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    let device = input.device.trim();
    if device.is_empty() || device.chars().count() > 100 {
        return Err(ApiError::BadRequest(
            "the device name must have 1 to 100 characters".into(),
        ));
    }
    let account = Account {
        handle: owner.user_id,
        email: owner.email,
        name: owner.display_name,
    };
    let created = issue(&state, &account, device).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// Encodes bytes as URL-safe base64 without padding.
fn base64_url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::api_tokens;

    fn owner() -> TokenOwner {
        TokenOwner {
            user_id: "alex".into(),
            email: Some("alex@example.com".into()),
            display_name: Some("Alex".into()),
        }
    }

    /// Stores a code the way `login` does, valid until `expires_at` (Unix seconds).
    async fn issue_code(state: &AppState, code: &str, expires_at: i64) {
        let now = chrono::Utc::now().timestamp();
        cli_logins::create(&state.conn, &hash_token(code), &owner(), expires_at, now)
            .await
            .unwrap();
    }

    fn valid_until() -> i64 {
        chrono::Utc::now().timestamp() + 120
    }

    #[tokio::test]
    async fn a_code_becomes_a_token_once() {
        let state = AppState::in_memory("alex").await;
        issue_code(&state, "code-1", valid_until()).await;

        let request = || CliTokenRequest {
            code: "code-1".into(),
            device: "CLI on laptop".into(),
        };
        let (status, Json(created)) = exchange(State(state.clone()), Json(request()))
            .await
            .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert!(created.token.starts_with("trx_"));
        assert!(matches!(
            exchange(State(state.clone()), Json(request())).await,
            Err(ApiError::Unauthorized)
        ));

        let stored = api_tokens::owner_of(&state.conn, &hash_token(&created.token), 0)
            .await
            .unwrap()
            .expect("the token is stored");
        assert_eq!(stored.user_id, "alex");
    }

    #[tokio::test]
    async fn an_expired_or_unknown_code_is_refused() {
        let state = AppState::in_memory("alex").await;
        issue_code(&state, "old", chrono::Utc::now().timestamp() - 1).await;
        for code in ["old", "never-issued"] {
            let result = exchange(
                State(state.clone()),
                Json(CliTokenRequest {
                    code: code.into(),
                    device: "CLI".into(),
                }),
            )
            .await;
            assert!(matches!(result, Err(ApiError::Unauthorized)), "code {code}");
        }
    }

    #[tokio::test]
    async fn a_code_works_on_any_app_instance_sharing_the_database() {
        // Two server processes share one database during a rolling update.
        let first = AppState::in_memory("alex").await;
        issue_code(&first, "shared", valid_until()).await;
        let second = AppState::new(first.conn.clone());

        let result = exchange(
            State(second),
            Json(CliTokenRequest {
                code: "shared".into(),
                device: "CLI".into(),
            }),
        )
        .await;
        assert!(result.is_ok());
    }
}
