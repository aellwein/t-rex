use axum::{Json, extract::State};
use dto::Me;

use super::{AppState, CurrentAccount};

/// The signed-in account. Only the email is shown; data is keyed by the user handle.
pub async fn get(
    State(_state): State<AppState>,
    CurrentAccount(account): CurrentAccount,
) -> Json<Me> {
    Json(Me {
        email: account.email,
        name: account.name,
    })
}
