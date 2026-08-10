use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

pub type ApiResult<T = Response> = Result<T, AppError>;

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    msg: String,
}

#[derive(Debug, Serialize)]
struct FailureResponse {
    success: bool,
    error: ErrorDetail,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("User with ID '{0}' not found")]
    UserNotFound(Uuid),

    #[error("Upstream provider error ({provider}): {message}")]
    UpstreamApi { provider: String, message: String },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Invalid parameter: {0}")]
    InvalidParams(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::UserNotFound(_) => "USER_NOT_FOUND",
            AppError::UpstreamApi { .. } => "UPSTREAM_API_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::HttpClient(_) => "HTTP_CLIENT_ERROR",
            AppError::InvalidParams(_) => "INVALID_PARAMS",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::UserNotFound(_) => StatusCode::NOT_FOUND,
            AppError::UpstreamApi { .. } => StatusCode::BAD_GATEWAY,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::HttpClient(_) => StatusCode::BAD_GATEWAY,
            AppError::InvalidParams(_) => StatusCode::BAD_REQUEST,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        error!(error = ?self, "API request failed");
        let body = Json(FailureResponse {
            success: false,
            error: ErrorDetail {
                code: self.error_code().to_string(),
                msg: self.to_string(),
            },
        });

        (status, body).into_response()
    }
}
