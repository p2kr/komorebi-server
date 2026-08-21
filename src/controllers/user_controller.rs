use loco_rs::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::{
    adapters::MediaClientParams,
    controllers::success,
    models::{
        _entities::users::{self},
        media::MediaProvider,
        users::User,
    },
};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct LoginParams {
    pub username: String,
    pub passcode: Option<String>,
    pub provider: MediaProvider,
    pub is_sandbox: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetUserParams {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteUserParams {
    pub user_id: Uuid,
}

#[debug_handler]
async fn login(State(ctx): State<AppContext>, Json(params): Json<LoginParams>) -> Result<Response> {
    let user_res = User::find_by_username_and_provider_and_sandbox(
        &ctx.db,
        &params.username,
        params.provider,
        params.is_sandbox.unwrap_or(false),
    )
    .await;

    let user = match user_res {
        Ok(user) => user,
        Err(_) => return unauthorized("User not found"),
    };

    if !user.verify_passcode(params.passcode.as_deref()) {
        return unauthorized("Invalid passcode");
    }

    success(user)
}

#[debug_handler]
async fn save_user(State(ctx): State<AppContext>, Json(user): Json<User>) -> Result<Response> {
    debug!("saving user {:?}", user.username);
    // check if user exists
    let user = validate_user(&ctx.shared_store.get::<Client>().unwrap(), &user).await?;

    debug!("User validated: {:?}", user.username);

    let user = User::save_user(&ctx.db, user).await?;
    debug!(user_id = %user.id, "User successfully saved");
    success(user)
}

#[debug_handler]
async fn get_all_users(State(ctx): State<AppContext>) -> Result<Response> {
    let all_users = User::get_all_users(&ctx.db).await?;
    success(all_users)
}

#[debug_handler]
async fn get_user_by_id(
    State(ctx): State<AppContext>,
    Json(params): Json<GetUserParams>,
) -> Result<Response> {
    let user = users::Model::find_by_id(&ctx.db, params.user_id).await?;
    success(user)
}

#[debug_handler]
async fn delete_user_by_id(
    State(ctx): State<AppContext>,
    Json(params): Json<DeleteUserParams>,
) -> Result<Response> {
    let user = users::Model::find_by_id(&ctx.db, params.user_id).await?;
    user.delete(&ctx.db).await?;
    success(params.user_id)
}

async fn validate_user(client: &Client, params: &User) -> Result<User> {
    let mut user = User {
        username: params.username.clone(),
        provider: params.provider.clone(),
        access_token: params.access_token.clone(),
        ..Default::default()
    };

    let media_client = user.provider.new_client(&client, &user);

    if let Some(token) = &user.access_token {
        // fetch username and avatar url
        debug!("Fetching username and avatar url for user");
        user = media_client.validate_new_user(token).await?;
        return Ok(user);
    }

    let media_client_params = MediaClientParams {
        limit: Some(1),
        ..Default::default()
    };

    if media_client
        .get_anime_list(&media_client_params)
        .await
        .is_ok()
    {
        debug!("validated user by anime: username={:?}", params.username);
        return Ok(user);
    }
    media_client.get_manga_list(&media_client_params).await?;

    debug!("validated user by manga: username={:?}", params.username);

    Ok(user)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/user")
        .add("/login", post(login))
        .add("/add", post(save_user))
        .add("/all", post(get_all_users))
        .add("/one", post(get_user_by_id))
        .add("/delete", post(delete_user_by_id))
}
