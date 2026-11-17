//! Holidays: list, create, edit, delete, and import from an iCalendar (`.ics`) file, either by
//! address or by pasted content.

use std::{collections::BTreeMap, str::FromStr, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{Duration as Days, NaiveDate};
use dto::{
    Holiday, HolidayImport, HolidayImportResult, HolidayImportText, HolidayInput, Page, PageQuery,
};
use icalendar::{Calendar, Component};

use super::{ApiError, ApiResult, AppState, CurrentUser};
use crate::store::holidays as store;

/// Largest calendar file accepted, from an address or pasted.
/// Largest calendar file accepted, from an address or pasted: 5 MB.
const MAX_ICS_BYTES: usize = 5 * 1024 * 1024;

/// Longest multi-day event that is expanded into single holidays.
/// Most days a multi-day event adds as holidays. A longer event adds only its first 366 days.
const MAX_SPAN_DAYS: i64 = 366;

/// Lists the holidays of the user, one page at a time. See [`PageQuery`] for the paging parameters.
pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<PageQuery>,
) -> ApiResult<Json<Page<Holiday>>> {
    let window = super::window(&query)?;
    Ok(Json(store::list(&state.conn, &user, window).await?))
}

/// Creates a holiday. Answers `201 Created`, or `409 Conflict` if the day already is a holiday of the user.
pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(mut input): Json<HolidayInput>,
) -> ApiResult<(StatusCode, Json<Holiday>)> {
    input.normalize();
    input.validate().map_err(ApiError::BadRequest)?;
    ensure_free_day(&state, &user, &input, None).await?;
    let holiday = store::create(&state.conn, &user, &input).await?;
    Ok((StatusCode::CREATED, Json(holiday)))
}

/// Returns one holiday of the user, or `404 Not Found`.
pub async fn get_one(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> ApiResult<Json<Holiday>> {
    store::get(&state.conn, &user, id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// Replaces a holiday. The day may stay the same. Answers `409` if another holiday is already on the new day.
pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(mut input): Json<HolidayInput>,
) -> ApiResult<Json<Holiday>> {
    input.normalize();
    input.validate().map_err(ApiError::BadRequest)?;
    ensure_free_day(&state, &user, &input, Some(id)).await?;
    store::update(&state.conn, &user, id, &input)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// Deletes a holiday. Answers `204 No Content`, or `404` if the holiday is not the user's.
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

/// Fetches an `.ics` address and adds its all-day events as holidays.
pub async fn import(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(input): Json<HolidayImport>,
) -> ApiResult<Json<HolidayImportResult>> {
    let url = reqwest::Url::parse(input.url.trim())
        .map_err(|_| ApiError::BadRequest("the address is not a valid URL".into()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ApiError::BadRequest(
            "the address must start with http:// or https://".into(),
        ));
    }
    let text = fetch_calendar(url).await?;
    add_from_calendar(&state, &user, &text).await.map(Json)
}

/// Adds the all-day events of pasted calendar content as holidays. Same rules as [`import`].
pub async fn import_text(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(input): Json<HolidayImportText>,
) -> ApiResult<Json<HolidayImportResult>> {
    if input.text.len() > MAX_ICS_BYTES {
        return Err(ApiError::BadRequest(
            "the calendar file is too large".into(),
        ));
    }
    add_from_calendar(&state, &user, &input.text)
        .await
        .map(Json)
}

/// Parses calendar content and adds its days. Days that already have a holiday keep it,
/// so edits made by hand survive a second import.
async fn add_from_calendar(
    state: &AppState,
    user: &str,
    text: &str,
) -> ApiResult<HolidayImportResult> {
    let days = parse_ics(text)
        .map_err(|_| ApiError::BadRequest("the file is not a valid calendar".into()))?;
    let (added, existing) = store::add_missing(&state.conn, user, &days).await?;
    Ok(HolidayImportResult { added, existing })
}

/// Reads the all-day events of an iCalendar file as `(date, name)` pairs, sorted by date.
///
/// Uses `DTSTART`, `DTEND` (exclusive, so a two-day event gives two days) and `SUMMARY`.
/// Recurrence rules are not expanded. Timed events count on the day they start.
pub fn parse_ics(text: &str) -> Result<Vec<(NaiveDate, String)>, String> {
    let calendar = Calendar::from_str(text)?;
    let mut found: BTreeMap<NaiveDate, String> = BTreeMap::new();
    for event in calendar.events() {
        let Some(start) = event.get_start().map(|value| value.date_naive()) else {
            continue;
        };
        let end = event
            .get_end()
            .map(|value| value.date_naive())
            .filter(|end| *end > start)
            .unwrap_or(start + Days::days(1));
        let span: i64 = (end - start).num_days().min(MAX_SPAN_DAYS);
        let label = event
            .get_summary()
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .unwrap_or("Holiday")
            .to_string();
        for offset in 0..span {
            let day = start + Days::days(offset);
            found.entry(day).or_insert_with(|| label.clone());
        }
    }
    Ok(found.into_iter().collect())
}

/// Downloads the calendar file. Fails with a message the user can act on.
async fn fetch_calendar(url: reqwest::Url) -> ApiResult<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|err| {
            log::error!("http client: {err}");
            ApiError::Internal
        })?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| ApiError::BadRequest("the address could not be reached".into()))?;
    if !response.status().is_success() {
        return Err(ApiError::BadRequest(format!(
            "the address answered with HTTP {}",
            response.status().as_u16()
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| ApiError::BadRequest("the calendar could not be read".into()))?;
    if bytes.len() > MAX_ICS_BYTES {
        return Err(ApiError::BadRequest(
            "the calendar file is too large".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Rejects a second holiday on the same day. `except` is the holiday being edited.
/// Rejects a second holiday on the same day. `except` is the holiday being edited, so it may keep its day.
async fn ensure_free_day(
    state: &AppState,
    user: &str,
    input: &HolidayInput,
    except: Option<i64>,
) -> ApiResult<()> {
    if store::id_on(&state.conn, user, input.date, except)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict("this day already is a holiday".into()));
    }
    Ok(())
}

#[cfg(test)]
mod ics_tests {
    use super::*;

    fn d(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    #[test]
    fn all_day_events_become_holidays() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20260101\r\nSUMMARY:Neujahr\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        assert_eq!(
            parse_ics(ics).unwrap(),
            vec![(d(1, 1), "Neujahr".to_string())]
        );
    }

    #[test]
    fn multi_day_events_are_expanded_with_exclusive_end() {
        let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;VALUE=DATE:20261224\nDTEND;VALUE=DATE:20261227\nSUMMARY:Christmas\nEND:VEVENT\nEND:VCALENDAR";
        let days: Vec<NaiveDate> = parse_ics(ics)
            .unwrap()
            .into_iter()
            .map(|(day, _)| day)
            .collect();
        assert_eq!(days, vec![d(12, 24), d(12, 25), d(12, 26)]);
    }

    #[test]
    fn folded_lines_and_escapes_are_read() {
        let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART:20261003\nSUMMARY:Day of German\n  Unity\\, Federal\nEND:VEVENT\nEND:VCALENDAR";
        assert_eq!(
            parse_ics(ics).unwrap(),
            vec![(d(10, 3), "Day of German Unity, Federal".to_string())]
        );
    }

    #[test]
    fn timed_events_count_on_their_start_day_and_missing_summary_gets_a_name() {
        let ics =
            "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART:20261225T090000Z\nEND:VEVENT\nEND:VCALENDAR";
        assert_eq!(
            parse_ics(ics).unwrap(),
            vec![(d(12, 25), "Holiday".to_string())]
        );
    }

    #[test]
    fn text_that_is_not_a_calendar_is_an_error_or_empty() {
        let result = parse_ics("<html>not a calendar</html>");
        assert!(result.map(|days| days.is_empty()).unwrap_or(true));
    }
}
