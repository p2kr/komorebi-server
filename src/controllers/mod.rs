use axum::{
    response::{IntoResponse, Response},
    Json,
};
use loco_rs::{prelude::format, Result};
use reqwest::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
struct SuccessResponse<T> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct FailureResponse {
    success: bool,
    error: String,
    description: Option<String>,
}

pub fn success<T: Serialize>(data: T) -> Result<Response> {
    format::json(SuccessResponse {
        success: true,
        data,
    })
}

pub fn fail(status_code: StatusCode, error: &str, description: Option<&str>) -> Result<Response> {
    Ok((
        status_code,
        Json(FailureResponse {
            success: false,
            error: error.into(),
            description: description.map(|m| m.into()),
        }),
    )
        .into_response())
}

pub mod user_controller;

pub mod media_controller;

pub mod crawler_controller;

pub mod vault_controller;
