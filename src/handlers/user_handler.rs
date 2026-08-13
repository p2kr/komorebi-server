use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::json;
use tracing::debug;
use uuid::Uuid;

use crate::{
    core::{ApiResult, AppState},
    handlers::success,
    models::user::User,
    services::{
        media_service::{MediaService, ValidateUserParams},
        user_service::UserService,
    },
};

#[derive(Debug, Deserialize)]
pub struct GetUserParams {
    pub user_id: Uuid,
}

pub async fn save_user(State(state): State<AppState>, Json(params): Json<User>) -> ApiResult {
    debug!("saving user {:#?}", params.username);
    // check if user exists by getting media
    let user = MediaService::validate_user(
        &state,
        &ValidateUserParams {
            username: params.username.clone(),
            provider: Some(params.provider.clone()),
            access_token: params.access_token.clone(),
        },
    )
    .await?;

    debug!("User validated: {:?}", user.username);

    let user = UserService::save_user(&state, user).await?;
    debug!(user_id = %user.id, "User successfully saved");
    Ok(success(user))
}

pub async fn get_all_users(State(state): State<AppState>) -> ApiResult {
    let users = UserService::get_all_users(&state).await?;
    debug!("fetched users: {:?}", users.len());
    Ok(success(users))
}

pub async fn get_user_by_id(
    State(state): State<AppState>,
    Json(params): Json<GetUserParams>,
) -> ApiResult {
    let user = UserService::get_user_by_id(&state, params.user_id).await?;
    debug!("fetched user: {:?}", user.id);
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
    debug!("deleted user: {}", params.user_id);
    Ok(success(json!({ "user_id": &params.user_id })))
}
