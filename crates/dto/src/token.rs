//! API tokens: long-lived credentials for the command line client. A token is shown once, when it
//! is created; the server keeps only a hash of it.

use serde::{Deserialize, Serialize};

/// A token as listed: no secret value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Id of the token, used to revoke it.
    pub id: i64,
    /// What the token is for, for example "CLI on laptop".
    pub name: String,
    /// RFC 3339 timestamp of creation.
    pub created_at: String,
    /// RFC 3339 timestamp after which the token is refused.
    pub expires_at: String,
}

/// A freshly created token. `token` is the only time the secret value is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatedToken {
    /// Id of the token, used to revoke it.
    pub id: i64,
    /// What the token is for, for example "CLI on laptop".
    pub name: String,
    /// RFC 3339 timestamp of creation.
    pub created_at: String,
    /// The secret value. It is returned only here: the server keeps a hash of it.
    pub token: String,
    /// RFC 3339 timestamp after which the token is refused.
    pub expires_at: String,
}

/// Exchanges the one-time code of a command line login for a token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliTokenRequest {
    /// The one-time code of the browser sign-in. It works once and expires after two minutes.
    pub code: String,
    /// Name for the new token, for example "CLI on laptop".
    pub device: String,
}

/// Request body for creating a token in the browser session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenInput {
    /// What the token is for, for example "CLI on laptop".
    pub name: String,
}
