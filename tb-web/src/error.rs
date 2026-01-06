//! Error types for the web API.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// API error response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Application error type.
#[derive(Debug)]
pub enum AppError {
    /// Resource not found.
    NotFound(String),
    /// Bad request (invalid parameters).
    BadRequest(String),
    /// TigerBeetle client error.
    Client(tb_rs::ClientError),
    /// Internal server error.
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Client(err) => {
                tracing::error!("TigerBeetle client error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "TigerBeetle client error".to_string(),
                )
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

impl From<tb_rs::ClientError> for AppError {
    fn from(err: tb_rs::ClientError) -> Self {
        AppError::Client(err)
    }
}
