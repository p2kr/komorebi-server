use axum::{
    response::{IntoResponse, Response},
    Json,
};
use loco_rs::{prelude::format, Result};
use reqwest::StatusCode;
use serde::Serialize;

pub mod user_controller;

#[derive(Serialize)]
struct SuccessResponse<T> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct FailureResponse {
    success: bool,
    msg: String,
}

pub fn success<T: Serialize>(data: T) -> Result<Response> {
    format::json(SuccessResponse {
        success: true,
        data,
    })
}

pub fn fail(status_code: StatusCode, msg: &str) -> Result<Response> {
    Ok((
        status_code,
        Json(FailureResponse {
            success: false,
            msg: msg.to_string(),
        }),
    )
        .into_response())
}
