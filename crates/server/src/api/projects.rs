use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use dto::{Page, PageQuery, Project, ProjectInput};

use super::{ApiError, ApiResult, AppState, CurrentUser};
use crate::store::projects as store;

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<PageQuery>,
) -> ApiResult<Json<Page<Project>>> {
    let window = super::window(&query)?;
    Ok(Json(store::list(&state.conn, &user, window).await?))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(mut input): Json<ProjectInput>,
) -> ApiResult<(StatusCode, Json<Project>)> {
    input.normalize();
    input.validate().map_err(ApiError::BadRequest)?;
    let project = store::create(&state.conn, &user, &input).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

pub async fn get_one(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> ApiResult<Json<Project>> {
    store::get(&state.conn, &user, id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(mut input): Json<ProjectInput>,
) -> ApiResult<Json<Project>> {
    input.normalize();
    input.validate().map_err(ApiError::BadRequest)?;
    store::update(&state.conn, &user, id, &input)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if store::delete(&state.conn, &user, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
