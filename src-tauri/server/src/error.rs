use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[source] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal Error: {0}")]
    Internal(String),
    #[error("Conflict: {0}")]
    Conflict(String),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        crate::db::note_stale_cached_plan(&e);
        AppError::Database(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_kind, client_message, log_message) = match &self {
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database",
                "Internal server error".to_string(),
                format!("{e:?}"),
            ),
            AppError::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "io",
                "Internal server error".to_string(),
                format!("{e:?}"),
            ),
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, "not_found", msg.clone(), msg.clone())
            }
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "Unauthorized".to_string(),
                "Unauthorized".to_string(),
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "bad_request",
                msg.clone(),
                msg.clone(),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                msg.clone(),
                msg.clone(),
            ),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone(), msg.clone()),
        };

        if status.is_server_error() {
            tracing::error!(
                target: "gotiga_server::error",
                status = status.as_u16(),
                error_kind,
                error = %log_message,
                "request failed"
            );
        } else {
            tracing::warn!(
                target: "gotiga_server::error",
                status = status.as_u16(),
                error_kind,
                error = %log_message,
                "request rejected"
            );
        }

        let body = Json(json!({
            "error": client_message,
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
