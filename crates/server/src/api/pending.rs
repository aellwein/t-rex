use axum::{Json, extract::State, http::StatusCode};
use dto::PendingStart;

use super::{ApiError, ApiResult, AppState, CurrentUser};
use crate::store::pending as store;

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> ApiResult<Json<Option<PendingStart>>> {
    Ok(Json(store::get(&state.conn, &user).await?))
}

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(input): Json<PendingStart>,
) -> ApiResult<Json<PendingStart>> {
    input.validate().map_err(ApiError::BadRequest)?;
    Ok(Json(store::put(&state.conn, &user, &input).await?))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> ApiResult<StatusCode> {
    store::delete(&state.conn, &user).await?;
    Ok(StatusCode::NO_CONTENT)
}
