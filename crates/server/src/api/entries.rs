use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::NaiveDate;
use dto::{EntryQuery, Page, PageQuery, TimeEntry, TimeEntryInput};

use super::{ApiError, ApiResult, AppState, CurrentUser};
use crate::store::{Table, entries as store, owned_exists};

/// Range used when a list request gives no `from` or `to`.
fn default_range() -> (NaiveDate, NaiveDate) {
    (
        NaiveDate::from_ymd_opt(1900, 1, 1).expect("valid date"),
        NaiveDate::from_ymd_opt(2999, 12, 31).expect("valid date"),
    )
}

/// Makes sure the referenced project and absence kind belong to the user.
async fn ensure_references(state: &AppState, user: &str, input: &TimeEntryInput) -> ApiResult<()> {
    if let Some(id) = input.project_id
        && !owned_exists(&state.conn, Table::Projects, id, user).await?
    {
        return Err(ApiError::BadRequest(format!("unknown project {id}")));
    }
    if let Some(id) = input.absence_kind_id
        && !owned_exists(&state.conn, Table::AbsenceKinds, id, user).await?
    {
        return Err(ApiError::BadRequest(format!("unknown absence kind {id}")));
    }
    Ok(())
}

/// Rejects a time record that overlaps another record of the same day.
async fn ensure_no_overlap(
    state: &AppState,
    user: &str,
    input: &TimeEntryInput,
    except: Option<i64>,
) -> ApiResult<()> {
    let overlapping = store::overlaps(
        &state.conn,
        user,
        input.date,
        input.start_minute,
        input.end_minute(),
        except,
    )
    .await?;
    if overlapping {
        return Err(ApiError::Conflict(
            "overlaps with another time record on this day".into(),
        ));
    }
    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<EntryQuery>,
    Query(page): Query<PageQuery>,
) -> ApiResult<Json<Page<TimeEntry>>> {
    let (default_from, default_to) = default_range();
    let from = query.from.unwrap_or(default_from);
    let to = query.to.unwrap_or(default_to);
    if from > to {
        return Err(ApiError::BadRequest("from must not be after to".into()));
    }
    let window = super::window(&page)?;
    Ok(Json(
        store::list(&state.conn, &user, from, to, window).await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(mut input): Json<TimeEntryInput>,
) -> ApiResult<(StatusCode, Json<TimeEntry>)> {
    input.normalize();
    input.validate().map_err(ApiError::BadRequest)?;
    ensure_references(&state, &user, &input).await?;
    ensure_no_overlap(&state, &user, &input, None).await?;
    let entry = store::create(&state.conn, &user, &input).await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

pub async fn get_one(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> ApiResult<Json<TimeEntry>> {
    store::get(&state.conn, &user, id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(mut input): Json<TimeEntryInput>,
) -> ApiResult<Json<TimeEntry>> {
    input.normalize();
    input.validate().map_err(ApiError::BadRequest)?;
    ensure_references(&state, &user, &input).await?;
    ensure_no_overlap(&state, &user, &input, Some(id)).await?;
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
