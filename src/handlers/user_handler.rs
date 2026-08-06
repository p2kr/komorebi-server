use axum::{Json, extract::State};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    core::{ApiResult, AppState},
    handlers::success,
    services::user_service::{Params, UserService},
};

#[derive(Debug, Deserialize)]
pub struct GetUserParams {
    pub user_id: Uuid,
}

pub async fn save_user(State(state): State<AppState>, Json(params): Json<Params>) -> ApiResult {
    let user = UserService::save_user(&state, params).await?;
    Ok(success(user))
}

pub async fn get_all_users(State(state): State<AppState>) -> ApiResult {
    let users = UserService::get_all_users(&state).await?;
    Ok(success(users))
}

pub async fn get_user_by_id(
    State(state): State<AppState>,
    Json(params): Json<GetUserParams>,
) -> ApiResult {
    let user = UserService::get_user_by_id(&state, params.user_id).await?;
    Ok(success(user))
}

#[derive(Debug, Deserialize)]
pub struct DeleteUserParams {
    pub user_id: Uuid,
}

pub async fn delete_user(
    State(state): State<AppState>,
    Json(params): Json<DeleteUserParams>,
) -> ApiResult {
    UserService::delete_user(&state, params.user_id).await?;
    Ok(success(serde_json::json!({ "deleted": true })))
}
