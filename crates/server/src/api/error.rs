//! Error responses. Each [`ApiError`] variant has one status code and a JSON body with the reason.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dto::ErrorBody;

/// The result of an API handler: its value, or the error that becomes the response.
pub type ApiResult<T> = Result<T, ApiError>;

/// Errors a handler can return. Each maps to one HTTP status code.
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound,
    Conflict(String),
    /// No valid sign-in for this request.
    Unauthorized,
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            ApiError::Conflict(message) => (StatusCode::CONFLICT, message),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "sign in required".to_string()),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
        };
        (status, Json(ErrorBody { error })).into_response()
    }
}

/// Maps database failures to API errors. The messages differ between SQLite and Postgres, so
/// both are matched here.
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        let message = format!("{err:#}");
        match constraint_error(&message) {
            Some(api_error) => api_error,
            None => {
                log::error!("database error: {message}");
                ApiError::Internal
            }
        }
    }
}

/// The API error for a constraint violation reported by the database, if the message is one.
fn constraint_error(message: &str) -> Option<ApiError> {
    let unique = message.contains("UNIQUE constraint failed")
        || message.contains("duplicate key value violates unique constraint");
    let foreign_key = message.contains("FOREIGN KEY constraint failed")
        || message.contains("violates foreign key constraint");
    let check = message.contains("CHECK constraint failed")
        || message.contains("violates check constraint");
    if unique {
        Some(ApiError::Conflict(
            "an item with this name already exists".into(),
        ))
    } else if foreign_key {
        Some(ApiError::Conflict(
            "still used by time entries; archive it instead".into(),
        ))
    } else if check {
        Some(ApiError::BadRequest("invalid value".into()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_and_postgres_constraint_messages_map_the_same_way() {
        for message in [
            "UNIQUE constraint failed: projects.user_id, projects.name",
            "duplicate key value violates unique constraint \"projects_user_id_name_key\"",
        ] {
            assert!(
                matches!(constraint_error(message), Some(ApiError::Conflict(_))),
                "{message}"
            );
        }
        for message in [
            "FOREIGN KEY constraint failed",
            "insert or update violates foreign key constraint \"time_entries_project_id_fkey\"",
        ] {
            assert!(
                matches!(constraint_error(message), Some(ApiError::Conflict(_))),
                "{message}"
            );
        }
        assert!(matches!(
            constraint_error("new row violates check constraint \"time_entries_minutes_check\""),
            Some(ApiError::BadRequest(_))
        ));
        assert!(constraint_error("connection refused").is_none());
    }
}
