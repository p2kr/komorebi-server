use axum::{Json, extract::State};
use tracing::debug;

use crate::{
    adapters::MediaClientParams,
    core::{ApiResult, AppState},
    handlers::success,
    services::media_service::MediaService,
};

pub async fn get_user_anime_list(
    State(state): State<AppState>,
    Json(params): Json<MediaClientParams>,
) -> ApiResult {
    let anime_list = MediaService::get_user_anime_list(&state, &params).await?;
    debug!("fetched anime_list: {:?}", anime_list.data.len());
    Ok(success(anime_list))
}

pub async fn get_user_manga_list(
    State(state): State<AppState>,
    Json(params): Json<MediaClientParams>,
) -> ApiResult {
    let manga_list = MediaService::get_user_manga_list(&state, &params).await?;
    debug!("fetched manga_list: {:?}", manga_list.data.len());
    Ok(success(manga_list))
}
