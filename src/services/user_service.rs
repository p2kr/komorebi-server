use uuid::Uuid;

use crate::{
    core::{AppError, AppState},
    db::user_repo::UserRepo,
    models::user::User,
};

pub struct UserService;

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
}
