use tracing::debug;

use crate::{
    adapters::{
        MediaClient, MediaClientParams, anilist_client::AniListClient, mal_client::MalClient,
    },
    core::{AppError, AppState},
    db::user_repo::UserRepo,
    models::media::{MediaProvider, PaginatedResponse},
};

pub struct MediaService;

impl MediaService {
    pub async fn get_user_anime_list(
        state: &AppState,
        provider: &MediaProvider,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        let user = UserRepo::new(state.db.clone())
            .fetch_user_by_id(params.user_id)
            .await?
            .ok_or(AppError::UserNotFound(params.user_id))?;

        let response = match provider {
            MediaProvider::MAL => {
                MalClient::get_anime_list(&state.http_client, &user, params).await?
            }
            MediaProvider::ANILIST => {
                AniListClient::get_anime_list(&state.http_client, &user, params).await?
            }
        };

        debug!(
            "found {:?} animes for user {:?} from provider {:?}",
            response.data.len(),
            params.user_id,
            provider
        );

        Ok(response)
    }

    pub async fn get_user_manga_list(
        state: &AppState,
        provider: &MediaProvider,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        let user = UserRepo::new(state.db.clone())
            .fetch_user_by_id(params.user_id)
            .await?
            .ok_or(AppError::UserNotFound(params.user_id))?;

        let response = match provider {
            MediaProvider::MAL => {
                MalClient::get_manga_list(&state.http_client, &user, params).await?
            }
            MediaProvider::ANILIST => {
                AniListClient::get_manga_list(&state.http_client, &user, params).await?
            }
        };

        debug!(
            "found {:?} mangas for user {:?} from provider {:?}",
            response.data.len(),
            params.user_id,
            provider
        );

        Ok(response)
    }
}
