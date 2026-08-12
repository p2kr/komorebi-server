pub mod media_handler;
pub mod user_handler;

use axum::{
    Json, Router,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, post},
};
use serde::Serialize;
use serde_json::json;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::debug;

use crate::{
    core::{ENV_CONFIGS, get_server_uptime, load_app_state},
    handlers::{
        media_handler::{get_user_anime_list, get_user_manga_list},
        user_handler::{delete_user, get_all_users, get_user_by_id, save_user},
    },
};

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

pub fn success<T: Serialize>(data: T) -> Response {
    (
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            data,
        }),
    )
        .into_response()
}

pub fn fail(status_code: Option<StatusCode>, error_code: &str, error_msg: &str) -> Response {
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
        .into_response()
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

fn get_cors_layer() -> CorsLayer {
    if !ENV_CONFIGS.cors_origins.is_empty() {
        let origins = ENV_CONFIGS
            .cors_origins
            .clone()
            .iter()
            .map(|x| x.parse().unwrap())
            .collect::<Vec<HeaderValue>>();

        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::permissive()
    }
}

/// Add all routes here
pub async fn make_routes() -> Router {
    let media = Router::new()
        .route("/anime", post(get_user_anime_list))
        .route("/manga", post(get_user_manga_list));

    let user = Router::new()
        .route("/add", post(save_user))
        .route("/all", post(get_all_users))
        .route("/one", post(get_user_by_id))
        .route("/delete", post(delete_user));

    let v1 = Router::new()
        .route("/", any(health_check))
        .route("/health", any(health_check))
        .nest("/media", media)
        .nest("/user", user);

    let router = Router::new()
        .route("/", any(health_check_bad))
        .nest("/api/v1", v1)
        .layer(TraceLayer::new_for_http())
        .layer(get_cors_layer())
        .with_state(load_app_state().await);

    debug!("registered routes: {:?}", router);

    router
}
