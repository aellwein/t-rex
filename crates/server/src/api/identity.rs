//! Who a request acts for, taken from the OIDC ID token that the auth layer keeps per session.

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_oidc_client::{extractors::OptionalIdToken, jwt::jwt_decoder::decode_jwt_unverified};

use super::{ApiError, AppState};
use crate::store;

/// The signed-in account of a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The key the account's data is stored under, see [`user_handle`].
    pub handle: String,
    /// The email address from the ID token, as the provider sent it.
    pub email: Option<String>,
    /// The display name from the ID token, as the provider sent it.
    pub name: Option<String>,
}

/// The account a request acts for, read from a browser session or a command line token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentAccount(pub Account);

/// The account a request acts for. A request is either a browser session (OIDC) or a command line
/// token sent as `Authorization: Bearer <token>`.
impl FromRequestParts<AppState> for CurrentAccount {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        #[cfg(test)]
        if let Some(user) = &state.test_user {
            return Ok(CurrentAccount(Account {
                handle: user.to_string(),
                email: None,
                name: None,
            }));
        }

        if let Some(token) = bearer_token(parts) {
            let owner = store::api_tokens::owner_of(
                &state.conn,
                &hash_token(token),
                chrono::Utc::now().timestamp(),
            )
            .await
            .map_err(ApiError::from)?
            .ok_or(ApiError::Unauthorized)?;
            return Ok(CurrentAccount(Account {
                handle: owner.user_id,
                email: owner.email,
                name: owner.display_name,
            }));
        }

        session_account(parts, state).await.map(CurrentAccount)
    }
}

/// The account of a browser session only. Used where a token must not be enough, such as
/// creating new tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAccount(pub Account);

impl FromRequestParts<AppState> for SessionAccount {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        #[cfg(test)]
        if let Some(user) = &state.test_user {
            return Ok(SessionAccount(Account {
                handle: user.to_string(),
                email: None,
                name: None,
            }));
        }
        session_account(parts, state).await.map(SessionAccount)
    }
}

/// The account of the OIDC session the auth layer keeps for this request.
async fn session_account(parts: &mut Parts, state: &AppState) -> Result<Account, ApiError> {
    let token = OptionalIdToken::from_request_parts(parts, state)
        .await
        .ok()
        .and_then(|OptionalIdToken(token)| token)
        .ok_or(ApiError::Unauthorized)?;

    // The token was issued by the provider and kept server-side by the auth layer,
    // so its signature was checked at login and is not checked again here.
    let (_, claims) = decode_jwt_unverified(&token).map_err(|err| {
        log::warn!("ID token could not be read: {err:?}");
        ApiError::Unauthorized
    })?;

    Ok(Account {
        handle: user_handle(&claims.sub, claims.email.as_deref(), claims.email_verified),
        email: claims.email,
        name: claims.name,
    })
}

/// The token of an `Authorization: Bearer` header, if there is one.
fn bearer_token(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// The value stored for a token: its SHA-256 as hex. Tokens are random, so no salt is needed.
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The user a request acts for. Stored data is keyed by [`user_handle`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentUser(pub String);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let CurrentAccount(account) = CurrentAccount::from_request_parts(parts, state).await?;
        Ok(CurrentUser(account.handle))
    }
}

/// The key a user's data is stored under: the verified email address, lowercased.
///
/// Without a verified email the OIDC subject is used, so the account still gets its own data.
pub fn user_handle(sub: &str, email: Option<&str>, email_verified: Option<bool>) -> String {
    let email = email
        .map(|email| email.trim().to_lowercase())
        .filter(|email| !email.is_empty());
    match email {
        Some(email) if email_verified == Some(true) => email,
        _ => sub.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_email_is_the_handle() {
        assert_eq!(
            user_handle("sub-123", Some("Alex@Example.com"), Some(true)),
            "alex@example.com"
        );
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        assert_eq!(
            user_handle("sub-123", Some("  alex@example.com \n"), Some(true)),
            "alex@example.com"
        );
    }

    #[test]
    fn unverified_email_falls_back_to_subject() {
        assert_eq!(
            user_handle("sub-123", Some("alex@example.com"), Some(false)),
            "sub-123"
        );
        assert_eq!(
            user_handle("sub-123", Some("alex@example.com"), None),
            "sub-123"
        );
    }

    #[test]
    fn missing_or_blank_email_falls_back_to_subject() {
        assert_eq!(user_handle("sub-123", None, Some(true)), "sub-123");
        assert_eq!(user_handle("sub-123", Some("   "), Some(true)), "sub-123");
    }
}

#[cfg(test)]
mod token_tests {
    use super::*;
    use crate::store::{api_tokens, test_db};
    use axum::{extract::FromRequestParts, http::Request};

    async fn parts_with(authorization: Option<&str>) -> Parts {
        let mut builder = Request::builder().uri("/api/v1/projects");
        if let Some(value) = authorization {
            builder = builder.header("authorization", value);
        }
        builder.body(()).unwrap().into_parts().0
    }

    #[tokio::test]
    async fn a_stored_token_acts_as_its_owner() {
        let state = AppState::new(test_db().await);
        let token = "trx_example-token";
        api_tokens::create(
            &state.conn,
            api_tokens::NewToken {
                user_id: "alex@example.com",
                name: "CLI on laptop",
                email: Some("alex@example.com"),
                display_name: Some("Alex"),
                token_hash: &hash_token(token),
                created_at: "2026-11-11T08:00:00+00:00",
                expires_at: 4_102_444_800,
            },
        )
        .await
        .unwrap();

        let mut parts = parts_with(Some(&format!("Bearer {token}"))).await;
        let CurrentAccount(account) = CurrentAccount::from_request_parts(&mut parts, &state)
            .await
            .unwrap();
        assert_eq!(account.handle, "alex@example.com");
        assert_eq!(account.name.as_deref(), Some("Alex"));
    }

    #[tokio::test]
    async fn an_unknown_or_missing_token_is_unauthorized() {
        let state = AppState::new(test_db().await);
        for header in [Some("Bearer trx_unknown"), None] {
            let mut parts = parts_with(header).await;
            let result = CurrentAccount::from_request_parts(&mut parts, &state).await;
            assert!(matches!(result, Err(ApiError::Unauthorized)), "{header:?}");
        }
    }

    #[tokio::test]
    async fn a_token_cannot_create_tokens() {
        let state = AppState::new(test_db().await);
        let mut parts = parts_with(Some("Bearer trx_anything")).await;
        let result = SessionAccount::from_request_parts(&mut parts, &state).await;
        assert!(matches!(result, Err(ApiError::Unauthorized)));
    }

    #[test]
    fn token_hashes_are_stable_and_hide_the_token() {
        let hash = hash_token("trx_secret");
        assert_eq!(hash, hash_token("trx_secret"));
        assert_eq!(hash.len(), 64);
        assert!(!hash.contains("secret"));
    }
}
