//! Projects: list, create, edit and delete. A project that time records use cannot be deleted.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use dto::{Page, PageQuery, Project, ProjectInput};

use super::{ApiError, ApiResult, AppState, CurrentUser};
use crate::store::projects as store;

/// Lists the projects of the signed-in user, one page at a time. See [`PageQuery`] for the paging parameters.
pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<PageQuery>,
) -> ApiResult<Json<Page<Project>>> {
    let window = super::window(&query)?;
    Ok(Json(store::list(&state.conn, &user, window).await?))
}

/// Creates a project with a name and a colour. The input is trimmed and checked first. Answers `201 Created` with the new project.
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

/// Returns one project of the user, or `404 Not Found`.
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

/// Replaces a project with the given state. Answers `404` if the project is not the user's and `409` if the new name is already taken.
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

/// Deletes a project. Answers `204 No Content`. A project that time records use cannot be deleted; the database refuses it and the answer is `409 Conflict`, so archive it instead.
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
