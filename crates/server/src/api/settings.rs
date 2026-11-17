//! Settings: read and save. Saved values are validated first.

use axum::{Json, extract::State};
use dto::Settings;

use super::{ApiError, ApiResult, AppState, CurrentUser};
use crate::store::settings as store;

/// Returns the settings of the user. Settings that were never saved have their defaults.
pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> ApiResult<Json<Settings>> {
    Ok(Json(store::get(&state.conn, &user).await?))
}

/// Saves the settings and returns them. Answers `400` for an unknown theme or a value out of range.
pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(input): Json<Settings>,
) -> ApiResult<Json<Settings>> {
    input.validate().map_err(ApiError::BadRequest)?;
    Ok(Json(store::put(&state.conn, &user, &input).await?))
}
