//! HTTP client for the t-rex REST API.
//!
//! Paths are relative, so the app talks to the server it was loaded from.

use std::fmt;

use chrono::NaiveDate;
use dto::{
    AbsenceKind, AbsenceKindInput, ErrorBody, Holiday, HolidayImport, HolidayImportResult,
    HolidayImportText, HolidayInput, Me, Page, PendingStart, Project, ProjectInput, Settings,
    TimeEntry, TimeEntryInput, TokenInfo, VersionInfo, MAX_LIMIT,
};
use gloo_net::http::{Request, Response};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Why a request failed. The message is shown to the user, as it is.
pub enum ApiError {
    /// No response at all: offline, server down, or a redirect to the login page.
    Network(String),
    /// The server answered with an error status and a message.
    Status { code: u16, message: String },
    /// The body did not have the expected shape.
    Decode(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Network(message) => write!(f, "network error ({message})"),
            ApiError::Status { message, .. } => write!(f, "{message}"),
            ApiError::Decode(message) => write!(f, "unexpected response ({message})"),
        }
    }
}

impl From<gloo_net::Error> for ApiError {
    fn from(err: gloo_net::Error) -> Self {
        ApiError::Network(err.to_string())
    }
}

/// The result of an API call: its value, or the reason it failed.
pub type ApiResult<T> = Result<T, ApiError>;

/// Reads a JSON answer. An error status becomes an [`ApiError::Status`] with the server's message.
async fn parse<T: DeserializeOwned>(response: Response) -> ApiResult<T> {
    if !response.ok() {
        return Err(status_error(response).await);
    }
    response
        .json::<T>()
        .await
        .map_err(|err| ApiError::Decode(err.to_string()))
}

/// Reads every item of a list, one page at a time. `path` may already carry a query string.
async fn list_all<T: DeserializeOwned>(path: &str) -> ApiResult<Vec<T>> {
    let separator = if path.contains('?') { '&' } else { '?' };
    let mut items = Vec::new();
    loop {
        let url = format!("{path}{separator}limit={MAX_LIMIT}&offset={}", items.len());
        let page: Page<T> = parse(Request::get(&url).send().await?).await?;
        let more = page.has_more() && !page.items.is_empty();
        items.extend(page.items);
        if !more {
            return Ok(items);
        }
    }
}

/// Checks that a request without an answer body succeeded.
async fn expect_success(response: Response) -> ApiResult<()> {
    if response.ok() {
        Ok(())
    } else {
        Err(status_error(response).await)
    }
}

/// The error for an error status: the message the server sent, or the status code if it sent none.
async fn status_error(response: Response) -> ApiError {
    let code = response.status();
    let message = match response.json::<ErrorBody>().await {
        Ok(body) => body.error,
        Err(_) => format!("HTTP {code}"),
    };
    ApiError::Status { code, message }
}

// Projects

/// Lists all projects, reading every page from the server.
pub async fn list_projects() -> ApiResult<Vec<Project>> {
    list_all("/api/v1/projects").await
}

/// Creates a project and returns it.
pub async fn create_project(input: &ProjectInput) -> ApiResult<Project> {
    let request = Request::post("/api/v1/projects").json(input)?;
    parse(request.send().await?).await
}

/// Replaces a project with the given state and returns it.
pub async fn update_project(id: i64, input: &ProjectInput) -> ApiResult<Project> {
    let request = Request::put(&format!("/api/v1/projects/{id}")).json(input)?;
    parse(request.send().await?).await
}

/// Deletes a project. The server refuses it while time records use it, and that message is returned.
pub async fn delete_project(id: i64) -> ApiResult<()> {
    let response = Request::delete(&format!("/api/v1/projects/{id}"))
        .send()
        .await?;
    expect_success(response).await
}

// Absence kinds

/// Lists all absence kinds, reading every page from the server.
pub async fn list_absence_kinds() -> ApiResult<Vec<AbsenceKind>> {
    list_all("/api/v1/absence-kinds").await
}

/// Creates an absence kind and returns it.
pub async fn create_absence_kind(input: &AbsenceKindInput) -> ApiResult<AbsenceKind> {
    let request = Request::post("/api/v1/absence-kinds").json(input)?;
    parse(request.send().await?).await
}

/// Replaces an absence kind with the given state and returns it.
pub async fn update_absence_kind(id: i64, input: &AbsenceKindInput) -> ApiResult<AbsenceKind> {
    let request = Request::put(&format!("/api/v1/absence-kinds/{id}")).json(input)?;
    parse(request.send().await?).await
}

/// Deletes an absence kind. The server refuses it while time records use it, and that message is returned.
pub async fn delete_absence_kind(id: i64) -> ApiResult<()> {
    let response = Request::delete(&format!("/api/v1/absence-kinds/{id}"))
        .send()
        .await?;
    expect_success(response).await
}

// Holidays

/// Lists all holidays, reading every page from the server.
pub async fn list_holidays() -> ApiResult<Vec<Holiday>> {
    list_all("/api/v1/holidays").await
}

/// Creates a holiday and returns it.
pub async fn create_holiday(input: &HolidayInput) -> ApiResult<Holiday> {
    let request = Request::post("/api/v1/holidays").json(input)?;
    parse(request.send().await?).await
}

/// Replaces a holiday with the given state and returns it.
pub async fn update_holiday(id: i64, input: &HolidayInput) -> ApiResult<Holiday> {
    let request = Request::put(&format!("/api/v1/holidays/{id}")).json(input)?;
    parse(request.send().await?).await
}

/// Deletes a holiday.
pub async fn delete_holiday(id: i64) -> ApiResult<()> {
    let response = Request::delete(&format!("/api/v1/holidays/{id}"))
        .send()
        .await?;
    expect_success(response).await
}

/// Imports the all-day events of an iCalendar address as holidays.
pub async fn import_holidays(url: &str) -> ApiResult<HolidayImportResult> {
    let body = HolidayImport {
        url: url.to_string(),
    };
    let request = Request::post("/api/v1/holidays/import").json(&body)?;
    parse(request.send().await?).await
}

/// Imports the all-day events of pasted iCalendar content as holidays.
pub async fn import_holiday_text(text: &str) -> ApiResult<HolidayImportResult> {
    let body = HolidayImportText {
        text: text.to_string(),
    };
    let request = Request::post("/api/v1/holidays/import-text").json(&body)?;
    parse(request.send().await?).await
}

// Time entries

/// Entries from `from` to `to`, both days included.
pub async fn list_entries(from: NaiveDate, to: NaiveDate) -> ApiResult<Vec<TimeEntry>> {
    list_all(&format!("/api/v1/time-entries?from={from}&to={to}")).await
}

/// Every entry of the account, for the reports.
pub async fn list_all_entries() -> ApiResult<Vec<TimeEntry>> {
    list_all("/api/v1/time-entries").await
}

/// Creates a time record and returns it. The server checks the overlap with the other records of the day.
pub async fn create_entry(input: &TimeEntryInput) -> ApiResult<TimeEntry> {
    let request = Request::post("/api/v1/time-entries").json(input)?;
    parse(request.send().await?).await
}

/// Replaces a time record with the given state and returns it.
pub async fn update_entry(id: i64, input: &TimeEntryInput) -> ApiResult<TimeEntry> {
    let request = Request::put(&format!("/api/v1/time-entries/{id}")).json(input)?;
    parse(request.send().await?).await
}

/// Deletes a time record.
pub async fn delete_entry(id: i64) -> ApiResult<()> {
    let response = Request::delete(&format!("/api/v1/time-entries/{id}"))
        .send()
        .await?;
    expect_success(response).await
}

// Pending start

/// The start of work that has no time record yet, if one was saved.
pub async fn get_pending_start() -> ApiResult<Option<PendingStart>> {
    parse(Request::get("/api/v1/pending-start").send().await?).await
}

/// Saves the pending start of the day and returns it.
pub async fn put_pending_start(pending: &PendingStart) -> ApiResult<PendingStart> {
    let request = Request::put("/api/v1/pending-start").json(pending)?;
    parse(request.send().await?).await
}

/// Clears the pending start of the user.
pub async fn delete_pending_start() -> ApiResult<()> {
    let response = Request::delete("/api/v1/pending-start").send().await?;
    expect_success(response).await
}

// Command line tokens

/// Every token of the account, without its secret.
pub async fn list_tokens() -> ApiResult<Vec<TokenInfo>> {
    list_all("/api/v1/tokens").await
}

/// Revokes a token. The computer that uses it is signed out.
pub async fn delete_token(id: i64) -> ApiResult<()> {
    let response = Request::delete(&format!("/api/v1/tokens/{id}"))
        .send()
        .await?;
    expect_success(response).await
}

// Build

/// The build the server runs, for the page footer.
pub async fn get_version() -> ApiResult<VersionInfo> {
    parse(Request::get("/api/v1/version").send().await?).await
}

// Account

/// The signed-in account, for the header. A `401` means the session has ended.
pub async fn get_me() -> ApiResult<Me> {
    parse(Request::get("/api/v1/me").send().await?).await
}

// Settings

/// The settings of the user.
pub async fn get_settings() -> ApiResult<Settings> {
    parse(Request::get("/api/v1/settings").send().await?).await
}

/// Saves the settings and returns what the server stored.
pub async fn save_settings(settings: &Settings) -> ApiResult<Settings> {
    let request = Request::put("/api/v1/settings").json(settings)?;
    parse(request.send().await?).await
}
