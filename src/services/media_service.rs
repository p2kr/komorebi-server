use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::{
    adapters::MediaClientParams,
    core::{AppError, AppState},
    db::user_repo::UserRepo,
    models::{
        media::{MediaProvider, PaginatedResponse},
        user::User,
    },
};

pub struct MediaService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateUserParams {
    pub username: String,
    pub provider: Option<MediaProvider>,
    pub access_token: Option<String>,
}

impl MediaService {
    pub async fn get_user_anime_list(
        state: &AppState,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        let user = UserRepo::new(state.db.clone())
            .fetch_user_by_id(params.user_id)
            .await?
            .ok_or(AppError::UserNotFound(params.user_id))?;

        let media_client = user.provider.new_client(&state.http_client, &user);
        let response = media_client.get_anime_list(params).await?;

        debug!(
            "found {:?} animes for user {:?} from provider {:?}",
            response.data.len(),
            params.user_id,
            user.provider
        );

        Ok(response)
    }

    pub async fn get_user_manga_list(
        state: &AppState,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        let user = UserRepo::new(state.db.clone())
            .fetch_user_by_id(params.user_id)
            .await?
            .ok_or(AppError::UserNotFound(params.user_id))?;

        let media_client = user.provider.new_client(&state.http_client, &user);
        let response = media_client.get_manga_list(params).await?;

        debug!(
            "found {:?} mangas for user {:?} from provider {:?}",
            response.data.len(),
            params.user_id,
            user.provider
        );

        Ok(response)
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

        if let Some(_access_token) = &user.access_token {
            // fetch username and avatar url
            debug!("Fetching username and avatar url for user");
            user = media_client.validate_new_user(_access_token).await?;
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
            debug!("validated user by anime {:?}", params);
            return Ok(user);
        }
        media_client.get_manga_list(&media_client_params).await?;

        debug!("validated user by manga {:?}", params);

        Ok(user)
    }
}
