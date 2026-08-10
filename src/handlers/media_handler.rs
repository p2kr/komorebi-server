use axum::{Json, extract::State};
use serde::Deserialize;
use tracing::debug;

use crate::{
    adapters::MediaClientParams,
    core::{ApiResult, AppState},
    handlers::success,
    models::media::MediaProvider,
    services::media_service::MediaService,
};

#[derive(Debug, Deserialize)]
pub struct Params {
    #[serde(flatten)]
    pub params: MediaClientParams,
    pub provider: Option<MediaProvider>,
}

pub async fn get_user_anime_list(
    State(state): State<AppState>,
    Json(params): Json<Params>,
) -> ApiResult {
    let provider = params.provider.unwrap_or(MediaProvider::MAL);
    let anime_list = MediaService::get_user_anime_list(&state, &provider, &params.params).await?;
    debug!("fetched anime_list: {:?}", anime_list.data.len());
    Ok(success(anime_list))
}

pub async fn get_user_manga_list(
    State(state): State<AppState>,
    Json(params): Json<Params>,
) -> ApiResult {
    let provider = params.provider.unwrap_or(MediaProvider::MAL);
    let manga_list = MediaService::get_user_manga_list(&state, &provider, &params.params).await?;
    debug!("fetched manga_list: {:?}", manga_list.data.len());
    Ok(success(manga_list))
}
