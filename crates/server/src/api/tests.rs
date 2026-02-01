//! HTTP-level tests: they drive the real router against an in-memory database.

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header::CONTENT_TYPE},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use super::{AppState, router};

async fn app() -> Router {
    router(AppState::in_memory("alex").await)
}

/// Sends one request and returns the status and the JSON body (`null` if none).
async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(json) => {
            request = request.header(CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn create_project(app: &Router, name: &str) -> i64 {
    let (status, body) = call(
        app,
        Method::POST,
        "/api/v1/projects",
        Some(json!({ "name": name })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    body["id"].as_i64().unwrap()
}

async fn create_absence_kind(app: &Router, name: &str) -> i64 {
    let (status, body) = call(
        app,
        Method::POST,
        "/api/v1/absence-kinds",
        Some(json!({ "name": name, "paid": true })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    body["id"].as_i64().unwrap()
}

#[tokio::test]
async fn health_reports_ok() {
    let app = app().await;
    let (status, _) = call(&app, Method::GET, "/api/v1/health", None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn creating_a_project_trims_the_name_and_applies_defaults() {
    let app = app().await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/projects",
        Some(json!({ "name": "  Website  " })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["name"], "Website");
    assert_eq!(body["color"], "#0d9488");
    assert_eq!(body["archived"], false);
    assert!(body["id"].as_i64().is_some());
}

#[tokio::test]
async fn blank_project_name_is_a_bad_request() {
    let app = app().await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/projects",
        Some(json!({ "name": "   " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().is_some());
}

#[tokio::test]
async fn invalid_project_color_is_a_bad_request() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/projects",
        Some(json!({ "name": "Web", "color": "teal" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_project_name_is_a_conflict() {
    let app = app().await;
    create_project(&app, "Website").await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/projects",
        Some(json!({ "name": "Website" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn unknown_project_is_not_found() {
    let app = app().await;
    let (status, body) = call(&app, Method::GET, "/api/v1/projects/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not found");
}

#[tokio::test]
async fn projects_are_listed_sorted_by_name() {
    let app = app().await;
    create_project(&app, "Zeta").await;
    create_project(&app, "alpha").await;
    let (status, body) = call(&app, Method::GET, "/api/v1/projects", None).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["alpha", "Zeta"]);
}

#[tokio::test]
async fn project_can_be_updated() {
    let app = app().await;
    let id = create_project(&app, "Old").await;
    let uri = format!("/api/v1/projects/{id}");

    let (status, body) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "name": "New", "color": "#FF0000", "archived": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "New");
    assert_eq!(body["color"], "#ff0000");
    assert_eq!(body["archived"], true);

    let (_, fetched) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(fetched, body);
}

#[tokio::test]
async fn project_can_be_deleted() {
    let app = app().await;
    let id = create_project(&app, "Temp").await;
    let uri = format!("/api/v1/projects/{id}");

    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn project_with_time_entries_cannot_be_deleted() {
    let app = app().await;
    let id = create_project(&app, "Busy").await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 540, "minutes": 60, "project_id": id })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/projects/{id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn absence_kind_crud_round_trip() {
    let app = app().await;
    let id = create_absence_kind(&app, "Vacation").await;
    let uri = format!("/api/v1/absence-kinds/{id}");

    let (status, body) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "name": "Vacation", "paid": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["paid"], false);

    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn project_entry_is_created_and_returned() {
    let app = app().await;
    let project = create_project(&app, "Website").await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({
            "date": "2026-11-05",
            "start_minute": 540,
            "minutes": 90,
            "project_id": project,
            "note": "  Review  "
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["date"], "2026-11-05");
    assert_eq!(body["minutes"], 90);
    assert_eq!(body["start_minute"], 540);
    assert_eq!(body["project_id"], project);
    assert!(body["absence_kind_id"].is_null());
    assert_eq!(body["note"], "Review");
}

#[tokio::test]
async fn entry_with_unknown_project_is_a_bad_request() {
    let app = app().await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(
            json!({ "date": "2026-11-05", "start_minute": 540, "minutes": 60, "project_id": 999 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("unknown project"));
}

#[tokio::test]
async fn entry_with_unknown_absence_kind_is_a_bad_request() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 540, "minutes": 480, "absence_kind_id": 999 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn entry_needs_exactly_one_reference() {
    let app = app().await;
    let project = create_project(&app, "Website").await;
    let absence = create_absence_kind(&app, "Vacation").await;

    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({
            "date": "2026-11-05",
            "start_minute": 540,
            "minutes": 60,
            "project_id": project,
            "absence_kind_id": absence
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 540, "minutes": 60 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn entries_can_be_listed_by_inclusive_date_range() {
    let app = app().await;
    let project = create_project(&app, "Website").await;
    for date in ["2026-11-02", "2026-12-01"] {
        call(
            &app,
            Method::POST,
            "/api/v1/time-entries",
            Some(
                json!({ "date": date, "start_minute": 540, "minutes": 30, "project_id": project }),
            ),
        )
        .await;
    }

    let (status, body) = call(
        &app,
        Method::GET,
        "/api/v1/time-entries?from=2026-11-01&to=2026-11-30",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let entries = body["items"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["date"], "2026-11-02");
}

#[tokio::test]
async fn inverted_date_range_is_a_bad_request() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::GET,
        "/api/v1/time-entries?from=2026-12-01&to=2026-11-01",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_date_in_query_is_rejected() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::GET,
        "/api/v1/time-entries?from=November",
        None,
    )
    .await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn entry_can_be_updated_and_deleted() {
    let app = app().await;
    let project = create_project(&app, "Website").await;
    let (_, created) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 540, "minutes": 30, "project_id": project })),
    )
    .await;
    let uri = format!("/api/v1/time-entries/{}", created["id"]);

    let (status, body) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "date": "2026-11-06", "start_minute": 540, "minutes": 45, "project_id": project, "note": "changed" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["date"], "2026-11-06");
    assert_eq!(body["minutes"], 45);

    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn settings_have_defaults_until_saved() {
    let app = app().await;
    let (status, body) = call(&app, Method::GET, "/api/v1/settings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["theme"], "system");
    assert_eq!(body["language"], "system");
    assert_eq!(body["daily_minutes"], 480);
}

#[tokio::test]
async fn settings_can_be_saved_and_read_back() {
    let app = app().await;
    let (status, body) = call(
        &app,
        Method::PUT,
        "/api/v1/settings",
        Some(json!({ "theme": "catppuccin-mocha", "language": "de", "daily_minutes": 420 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["theme"], "catppuccin-mocha");

    let (_, fetched) = call(&app, Method::GET, "/api/v1/settings", None).await;
    assert_eq!(fetched, body);
}

#[tokio::test]
async fn settings_reject_an_unknown_language() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/v1/settings",
        Some(json!({ "theme": "dark", "language": "xx", "daily_minutes": 420 })),
    )
    .await;
    assert!(status.is_client_error());
}

#[tokio::test]
async fn settings_reject_more_than_a_day() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/v1/settings",
        Some(json!({ "theme": "nord-light", "language": "en", "daily_minutes": 2000 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_json_is_a_client_error() {
    let app = app().await;
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/projects")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn me_reports_no_email_for_test_user() {
    let app = app().await;
    let (status, body) = call(&app, Method::GET, "/api/v1/me", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "email": null, "name": null }));
}

#[tokio::test]
async fn overlapping_time_records_are_rejected_but_adjacent_ones_fit() {
    let app = app().await;
    let project = create_project(&app, "Overlap").await;
    let record = |start: u32, minutes: u32| json!({ "date": "2026-11-05", "start_minute": start, "minutes": minutes, "project_id": project });

    // 09:00-10:00
    let (status, first) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(record(540, 60)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Starts inside the first block.
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(record(570, 60)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // Covers the first block completely.
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(record(480, 240)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // Touching at 10:00 is fine, so is a block on another day.
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(record(600, 30)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let other_day =
        json!({ "date": "2026-11-06", "start_minute": 540, "minutes": 60, "project_id": project });
    let (status, _) = call(&app, Method::POST, "/api/v1/time-entries", Some(other_day)).await;
    assert_eq!(status, StatusCode::CREATED);

    // Updating a record may keep its own time, but not move onto another one.
    let uri = format!("/api/v1/time-entries/{}", first["id"]);
    let (status, _) = call(&app, Method::PUT, &uri, Some(record(540, 45))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&app, Method::PUT, &uri, Some(record(590, 60))).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn a_block_past_midnight_is_rejected() {
    let app = app().await;
    let project = create_project(&app, "Night").await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 1400, "minutes": 60, "project_id": project })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_record_keeps_its_break_and_the_day_sums_them() {
    let app = app().await;
    let project = create_project(&app, "Breaks").await;
    let morning = json!({ "date": "2026-11-05", "start_minute": 480, "minutes": 240, "break_minutes": 0, "project_id": project });
    let afternoon = json!({ "date": "2026-11-05", "start_minute": 765, "minutes": 240, "break_minutes": 45, "project_id": project });
    let (status, created) = call(&app, Method::POST, "/api/v1/time-entries", Some(afternoon)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["break_minutes"], 45);
    assert_eq!(created["minutes"], 240);
    let (status, _) = call(&app, Method::POST, "/api/v1/time-entries", Some(morning)).await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn the_break_takes_room_in_the_day_for_overlap_checks() {
    let app = app().await;
    let project = create_project(&app, "Room").await;
    // 08:00 for 8h work with a 45 minute break runs until 16:45.
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 480, "minutes": 480, "break_minutes": 45, "project_id": project })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Starting at 16:30 collides with the break; 16:45 is fine.
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 990, "minutes": 30, "project_id": project })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 1005, "minutes": 30, "project_id": project })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn a_break_above_a_day_is_rejected() {
    let app = app().await;
    let project = create_project(&app, "Long break").await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/time-entries",
        Some(json!({ "date": "2026-11-05", "start_minute": 480, "minutes": 60, "break_minutes": 1441, "project_id": project })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn pending_start_is_saved_until_cleared() {
    let app = app().await;
    let (status, body) = call(&app, Method::GET, "/api/v1/pending-start", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_null());

    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/v1/pending-start",
        Some(json!({ "date": "2026-11-06", "start_minute": 480 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = call(&app, Method::GET, "/api/v1/pending-start", None).await;
    assert_eq!(body, json!({ "date": "2026-11-06", "start_minute": 480 }));

    let (status, _) = call(&app, Method::DELETE, "/api/v1/pending-start", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = call(&app, Method::GET, "/api/v1/pending-start", None).await;
    assert!(body.is_null());
}

#[tokio::test]
async fn pending_start_outside_the_day_is_rejected() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/v1/pending-start",
        Some(json!({ "date": "2026-11-06", "start_minute": 1440 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn version_reports_the_build() {
    let app = app().await;
    let (status, body) = call(&app, Method::GET, "/api/v1/version", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn every_interface_language_can_be_saved() {
    let app = app().await;
    for language in ["fr", "es", "it", "pt", "nl", "pl", "zh", "ja"] {
        let (status, body) = call(
            &app,
            Method::PUT,
            "/api/v1/settings",
            Some(json!({ "theme": "system", "language": language, "daily_minutes": 480 })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{language} should be accepted");
        assert_eq!(body["language"], language);
    }
}

#[tokio::test]
async fn carry_over_can_be_negative_and_is_read_back() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/v1/settings",
        Some(json!({ "theme": "system", "language": "de", "daily_minutes": 480, "carry_over_minutes": -90 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = call(&app, Method::GET, "/api/v1/settings", None).await;
    assert_eq!(body["carry_over_minutes"], -90);
}

#[tokio::test]
async fn holiday_crud_round_trip_with_one_holiday_per_day() {
    let app = app().await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/holidays",
        Some(json!({ "date": "2026-12-25", "name": "Christmas" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = body["id"].as_i64().unwrap();

    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/holidays",
        Some(json!({ "date": "2026-12-25", "name": "Again" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("/api/v1/holidays/{id}"),
        Some(json!({ "date": "2026-12-26", "name": "Boxing Day" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["date"], "2026-12-26");

    let (status, body) = call(&app, Method::GET, "/api/v1/holidays", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/holidays/{id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn blank_holiday_name_is_a_bad_request() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/holidays",
        Some(json!({ "date": "2026-01-01", "name": "  " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn holiday_import_only_accepts_web_addresses() {
    let app = app().await;
    for url in ["file:///etc/passwd", "not a url"] {
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/v1/holidays/import",
            Some(json!({ "url": url })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{url}");
    }
}

#[tokio::test]
async fn pasted_calendar_adds_holidays_and_keeps_existing_days() {
    let app = app().await;
    let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART;VALUE=DATE:20260101\nSUMMARY:Neujahr\nEND:VEVENT\nEND:VCALENDAR";
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/holidays/import-text",
        Some(json!({ "text": ics })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "added": 1, "existing": 0 }));

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/holidays/import-text",
        Some(json!({ "text": ics })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "added": 0, "existing": 1 }));
}

#[tokio::test]
async fn pasted_text_that_is_not_a_calendar_adds_nothing_or_fails() {
    let app = app().await;
    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/holidays/import-text",
        Some(json!({ "text": "hello" })),
    )
    .await;
    // Text without any event is an empty import, not a stored holiday.
    assert!(
        status == StatusCode::OK && body == json!({ "added": 0, "existing": 0 })
            || status == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn unknown_theme_is_a_bad_request() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::PUT,
        "/api/v1/settings",
        Some(json!({ "theme": "neon", "language": "en", "daily_minutes": 480 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn lists_are_pages_with_the_total_and_the_default_window() {
    let app = app().await;
    for name in ["Alpha", "Beta", "Gamma"] {
        create_project(&app, name).await;
    }
    let (status, body) = call(&app, Method::GET, "/api/v1/projects", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    assert_eq!(body["limit"], 50);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["items"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn consecutive_pages_cover_the_list_without_overlap() {
    let app = app().await;
    for name in ["e", "b", "d", "a", "c"] {
        create_project(&app, name).await;
    }
    let mut names = Vec::new();
    for offset in [0, 2, 4] {
        let uri = format!("/api/v1/projects?limit=2&offset={offset}");
        let (status, body) = call(&app, Method::GET, &uri, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["total"], 5);
        names.extend(
            body["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["name"].as_str().unwrap().to_string()),
        );
    }
    assert_eq!(names, vec!["a", "b", "c", "d", "e"]);
}

#[tokio::test]
async fn an_offset_past_the_end_gives_an_empty_page_with_the_total() {
    let app = app().await;
    create_project(&app, "Only").await;
    let (status, body) = call(&app, Method::GET, "/api/v1/projects?offset=10", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert!(body["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn a_limit_outside_one_to_two_hundred_is_a_bad_request() {
    let app = app().await;
    for uri in [
        "/api/v1/projects?limit=0",
        "/api/v1/projects?limit=201",
        "/api/v1/projects?limit=many",
        "/api/v1/time-entries?limit=0",
        "/api/v1/holidays?limit=201",
        "/api/v1/absence-kinds?limit=0",
        "/api/v1/tokens?limit=0",
    ] {
        let (status, _) = call(&app, Method::GET, uri, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
    }
}

#[tokio::test]
async fn the_unversioned_api_is_gone() {
    let app = app().await;
    let (status, _) = call(&app, Method::GET, "/api/projects", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn created_tokens_are_listed_for_the_browser_session() {
    let app = app().await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/tokens",
        Some(json!({ "name": "CLI on laptop" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = call(&app, Method::GET, "/api/v1/tokens", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["name"], "CLI on laptop");
    assert!(
        body["items"][0].get("token").is_none(),
        "the secret is never listed"
    );
}

#[tokio::test]
async fn tokens_expire_ten_days_after_creation() {
    let app = app().await;
    let (status, created) = call(
        &app,
        Method::POST,
        "/api/v1/tokens",
        Some(json!({ "name": "CLI on laptop" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, listed) = call(&app, Method::GET, "/api/v1/tokens", None).await;
    let item = &listed["items"][0];
    let created_at =
        chrono::DateTime::parse_from_rfc3339(item["created_at"].as_str().unwrap()).unwrap();
    let expires_at =
        chrono::DateTime::parse_from_rfc3339(item["expires_at"].as_str().unwrap()).unwrap();
    // `expires_at` is stored in whole seconds, so it can be up to a second earlier than the creation
    // time plus the lifetime.
    let lifetime = (expires_at - created_at).num_seconds();
    assert!(
        (crate::store::api_tokens::TOKEN_LIFETIME_SECS - 1
            ..=crate::store::api_tokens::TOKEN_LIFETIME_SECS)
            .contains(&lifetime),
        "lifetime was {lifetime} seconds"
    );
    assert_eq!(created["expires_at"], item["expires_at"]);
}

#[tokio::test]
async fn an_account_cannot_hold_more_than_fifteen_tokens() {
    let app = app().await;
    let mut ids = Vec::new();
    for n in 0..15 {
        let (status, created) = call(
            &app,
            Method::POST,
            "/api/v1/tokens",
            Some(json!({ "name": format!("CLI {n}") })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "token {n}");
        ids.push(created["id"].as_i64().unwrap());
    }

    let (status, body) = call(
        &app,
        Method::POST,
        "/api/v1/tokens",
        Some(json!({ "name": "one too many" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body["error"].as_str().unwrap().contains("15"));

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/v1/tokens/{}", ids[0]),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/v1/tokens",
        Some(json!({ "name": "replacement" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn a_repeated_token_name_gets_the_creation_time() {
    let app = app().await;
    let mut names = Vec::new();
    for _ in 0..2 {
        let (status, created) = call(
            &app,
            Method::POST,
            "/api/v1/tokens",
            Some(json!({ "name": "alex@laptop" })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        names.push(created["name"].as_str().unwrap().to_string());
    }
    assert_eq!(names[0], "alex@laptop");
    assert!(names[1].starts_with("alex@laptop ("), "got {}", names[1]);
    assert!(names[1].ends_with(')'));
}
