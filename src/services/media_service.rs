use tracing::debug;

use crate::{
    adapters::MediaClientParams,
    core::{AppError, AppState},
    db::user_repo::UserRepo,
    models::media::PaginatedResponse,
};

pub struct MediaService;

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
}
