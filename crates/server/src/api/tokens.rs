//! API tokens: list, create (browser session only) and revoke.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use dto::{CreatedToken, Page, PageQuery, TokenInfo, TokenInput};

use super::{ApiError, ApiResult, AppState, CurrentAccount, SessionAccount};
use crate::store::api_tokens;

/// Longest accepted name for a token, in characters.
const MAX_TOKEN_NAME: usize = 100;

pub async fn list(
    State(state): State<AppState>,
    CurrentAccount(account): CurrentAccount,
    Query(query): Query<PageQuery>,
) -> ApiResult<Json<Page<TokenInfo>>> {
    let window = super::window(&query)?;
    Ok(Json(
        api_tokens::list(&state.conn, &account.handle, window).await?,
    ))
}

/// Creates a token for the signed-in browser session. The secret is returned only here.
pub async fn create(
    State(state): State<AppState>,
    SessionAccount(account): SessionAccount,
    Json(input): Json<TokenInput>,
) -> ApiResult<(StatusCode, Json<CreatedToken>)> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > MAX_TOKEN_NAME {
        return Err(ApiError::BadRequest(format!(
            "the name must have 1 to {MAX_TOKEN_NAME} characters"
        )));
    }
    let created = issue(&state, &account, name).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentAccount(account): CurrentAccount,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if api_tokens::delete(&state.conn, &account.handle, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}

/// Length of " (YYYY-MM-DD HH:MM:SS)", appended to a name that is already taken.
const DATE_SUFFIX_LEN: usize = 22;

/// Creates a random token, stores its hash, and returns the token once.
pub async fn issue(
    state: &AppState,
    account: &super::identity::Account,
    name: &str,
) -> ApiResult<CreatedToken> {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let token = format!("trx_{}", base64_url(&bytes));
    let now = chrono::Utc::now();
    let created_at = now.to_rfc3339();
    // A name that is already taken gets the creation time, so the token list stays readable.
    let name = if api_tokens::name_taken(&state.conn, &account.handle, name).await? {
        let stem: String = name
            .chars()
            .take(MAX_TOKEN_NAME - DATE_SUFFIX_LEN)
            .collect();
        format!("{stem} ({})", now.format("%Y-%m-%d %H:%M:%S"))
    } else {
        name.to_string()
    };
    let stored = api_tokens::create_within_limit(
        &state.conn,
        api_tokens::NewToken {
            user_id: &account.handle,
            name: &name,
            email: account.email.as_deref(),
            display_name: account.name.as_deref(),
            token_hash: &super::identity::hash_token(&token),
            created_at: &created_at,
            expires_at: now.timestamp() + api_tokens::TOKEN_LIFETIME_SECS,
        },
        now.timestamp(),
    )
    .await?
    .ok_or_else(|| {
        ApiError::Conflict(format!(
            "an account can hold at most {} tokens; revoke one first",
            api_tokens::MAX_TOKENS_PER_USER
        ))
    })?;
    Ok(CreatedToken {
        id: stored.id,
        name: stored.name,
        created_at: stored.created_at,
        expires_at: stored.expires_at,
        token,
    })
}

fn base64_url(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
