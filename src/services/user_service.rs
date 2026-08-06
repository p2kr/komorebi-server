use serde::Deserialize;
use uuid::Uuid;

use crate::{
    core::{AppError, AppState},
    db::user_repo::UserRepo,
    models::{media::MediaProvider, user::User},
};

pub struct UserService;

#[derive(Clone, Deserialize)]
pub struct Params {
    pub username: String,
    pub avatar_url: Option<String>,
    pub provider: MediaProvider,
    pub access_token: Option<String>,
}

impl UserService {
    pub async fn save_user(state: &AppState, user: Params) -> Result<User, AppError> {
        let user_repo = UserRepo::new(state.db.clone());
        let user = user_repo
            .save_user(
                user.username,
                user.avatar_url,
                user.provider,
                user.access_token,
            )
            .await?;

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
}

