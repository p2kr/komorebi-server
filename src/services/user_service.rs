use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::{
    adapters::MediaClientParams,
    core::{AppError, AppState},
    db::user_repo::UserRepo,
    models::{media::MediaProvider, user::User},
};

pub struct UserService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateUserParams {
    pub username: String,
    pub provider: Option<MediaProvider>,
    pub access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExchangeOauthTokenParams {
    pub provider: MediaProvider,
    pub code: String,
    pub code_verifier: String,
}

impl UserService {
    pub async fn save_user(state: &AppState, user: User) -> Result<User, AppError> {
        let user_repo = UserRepo::new(state.db.clone());
        let user = user_repo.save_user(user).await?;

        Ok(user)
    }

    pub async fn get_all_users(state: &AppState) -> Result<Vec<User>, AppError> {
        let users = UserRepo::new(state.db.clone()).fetch_all_users().await?;

        Ok(users)
    }

    pub async fn get_user_by_id(state: &AppState, user_id: Uuid) -> Result<User, AppError> {
        let user = UserRepo::new(state.db.clone())
            .fetch_user_by_id(user_id)
            .await?
            .ok_or(AppError::UserNotFound(user_id))?;

        Ok(user)
    }

    pub async fn delete_user(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
        let repo = UserRepo::new(state.db.clone());
        let user = repo.fetch_user_by_id(user_id).await?;
        if user.is_none() {
            return Err(AppError::UserNotFound(user_id));
        }

        repo.delete_user(user_id).await?;
        Ok(())
    }

    pub async fn validate_user(
        state: &AppState,
        params: &ValidateUserParams,
    ) -> Result<User, AppError> {
        let mut user = User::new(
            params.username.clone(),
            None,
            None,
            params.provider,
            params.access_token.clone(),
        );

        let media_client = user.provider.new_client(&state.http_client, &user);

        if let Some(token) = &user.access_token {
            // fetch username and avatar url
            debug!("Fetching username and avatar url for user");
            user = media_client.validate_new_user(token).await?;
            return Ok(user);
        }

        let media_client_params = MediaClientParams {
            user_id: Uuid::now_v7(), // Dummy
            status: None,
            sort: None,
            limit: Some(1),
            offset: None,
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

    pub async fn exchange_oauth_token(
        state: &AppState,
        params: &ExchangeOauthTokenParams,
    ) -> Result<String, AppError> {
        let client = params
            .provider
            .new_client(&state.http_client, &User::default());

        let token = client
            .exchange_oauth_token(&params.code, &params.code_verifier)
            .await?;

        Ok(token)
    }
}
