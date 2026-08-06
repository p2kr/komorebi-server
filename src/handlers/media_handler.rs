use axum::{
    extract::{Query, State},
    response::Response,
};
use serde::Deserialize;

use crate::{
    adapters::MediaClientParams,
    core::{AppError, AppState},
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
    Query(params): Query<Params>,
) -> Result<Response, AppError> {
    let provider = params.provider.unwrap_or(MediaProvider::MAL);
    let anime_list = MediaService::get_user_anime_list(&state, &provider, &params.params).await?;
    Ok(success(anime_list))
}

pub async fn get_user_manga_list(
    State(state): State<AppState>,
    Query(params): Query<Params>,
) -> Result<Response, AppError> {
    let provider = params.provider.unwrap_or(MediaProvider::MAL);
    let manga_list = MediaService::get_user_manga_list(&state, &provider, &params.params).await?;
    Ok(success(manga_list))
}
