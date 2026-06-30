use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// A catch-all error that converts any `anyhow::Error` into a JSON 500 (or a
/// chosen status) so handlers can use `?` freely.
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl AppError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        AppError {
            status,
            message: message.into(),
        }
    }

    /// A 400 Bad Request with a user-facing message.
    pub fn bad_request(message: impl Into<String>) -> Self {
        AppError::new(StatusCode::BAD_REQUEST, message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into().to_string())
    }
}
