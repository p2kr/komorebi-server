use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;
use serde_json::json;
use tower_http::trace::TraceLayer;

use crate::utils::get_server_uptime;

#[derive(Serialize)]
struct SuccessResponse<T> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    msg: String,
}

#[derive(Serialize)]
struct FailureResponse {
    success: bool,
    error: ErrorDetail,
}

pub fn success<T: Serialize>(data: T) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            data,
        }),
    )
}

pub fn fail(
    status_code: Option<StatusCode>,
    error_code: &str,
    error_msg: &str,
) -> impl IntoResponse {
    (
        status_code.unwrap_or(StatusCode::BAD_REQUEST),
        Json(FailureResponse {
            success: false,
            error: ErrorDetail {
                code: error_code.to_string(),
                msg: error_msg.to_string(),
            },
        }),
    )
}

pub async fn health_check() -> impl IntoResponse {
    success(json!({
        "base_url": "/api/v1",
        "uptime": get_server_uptime(),
        "version": "1.0.0",
    }))
}

pub async fn health_check_bad() -> impl IntoResponse {
    fail(None, "base_api_url", "/api/v1")
}

/// Add all routes here
pub fn make_routes() -> Router {
    let v1 = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check));

    Router::new()
        .route("/", get(health_check_bad))
        .nest("/api/v1", v1)
        .layer(TraceLayer::new_for_http())
}
