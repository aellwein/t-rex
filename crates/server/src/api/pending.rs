//! The pending start: the time work started today, kept until the time record is added.

use axum::{Json, extract::State, http::StatusCode};
use dto::PendingStart;

use super::{ApiError, ApiResult, AppState, CurrentUser};
use crate::store::pending as store;

/// Returns the pending start of the user, or `null` if there is none.
pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> ApiResult<Json<Option<PendingStart>>> {
    Ok(Json(store::get(&state.conn, &user).await?))
}

/// Saves the pending start, replacing an earlier one, and returns it. Answers `400` if the time is not on the day.
pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(input): Json<PendingStart>,
) -> ApiResult<Json<PendingStart>> {
    input.validate().map_err(ApiError::BadRequest)?;
    Ok(Json(store::put(&state.conn, &user, &input).await?))
}

/// Clears the pending start. Answers `204 No Content`, also when there was none.
pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> ApiResult<StatusCode> {
    store::delete(&state.conn, &user).await?;
    Ok(StatusCode::NO_CONTENT)
}
