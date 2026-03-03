//! # Shared Error Types
//!
//! `AppError` enum cung cấp error handling thống nhất cho toàn bộ API.
//! Implement `IntoResponse` cho Axum để tự động serialize error ra JSON.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Error chung cho toàn bộ application.
/// Mỗi variant map tới HTTP status code + JSON error response.
#[derive(Debug)]
pub enum AppError {
    /// 400 Bad Request – input validation failed
    BadRequest(String),
    /// 401 Unauthorized – missing/invalid token
    Unauthorized(String),
    /// 403 Forbidden – authenticated nhưng không có quyền
    Forbidden(String),
    /// 404 Not Found
    NotFound(String),
    /// 409 Conflict – duplicate resource
    Conflict(String),
    /// 500 Internal Server Error – database error, etc.
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An internal error occurred".to_string(),
                )
            }
        };

        (
            status,
            Json(json!({
                "error": error_type,
                "message": message
            })),
        )
            .into_response()
    }
}

/// Convert sqlx::Error → AppError
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".into()),
            sqlx::Error::Database(ref db_err) => {
                // PostgreSQL unique violation = 23505
                if db_err.code().as_deref() == Some("23505") {
                    AppError::Conflict("Resource already exists".into())
                } else {
                    AppError::Internal(e.to_string())
                }
            }
            _ => AppError::Internal(e.to_string()),
        }
    }
}
