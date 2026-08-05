use axum::{extract::Query, response::Response};
use serde::Deserialize;
use tracing::debug;

use crate::{
    handlers::{
        clients::{
            MediaClient, MedialClientParams, anilist_client::AniListClient, mal_client::MalClient,
        },
        fail, success,
    },
    models::media::MediaProvider,
};

#[derive(Debug, Deserialize)]
pub struct Params {
    #[serde(flatten)]
    pub params: MedialClientParams,
    pub provider: Option<MediaProvider>,
}

pub async fn get_user_anime_list(Query(params): Query<Params>) -> Response {
    let provider = params.provider.as_ref().unwrap_or(&MediaProvider::MAL);
    let anime_list = match provider {
        MediaProvider::MAL => MalClient::get_anime_list(&params.params).await,
        MediaProvider::ANILIST => AniListClient::get_anime_list(&params.params).await,
    };

    match anime_list {
        Err(e) => fail(None, "FETCH_ANIME_FAILED", &e.to_string()),
        Ok(v) => {
            debug!(
                "found {:?} animes for {:?} from {:?}",
                v.data.len(),
                &params,
                provider
            );
            success(v)
        }
    }
}

pub async fn get_user_manga_list(Query(params): Query<Params>) -> Response {
    let provider = params.provider.as_ref().unwrap_or(&MediaProvider::MAL);
    let manga_list = match provider {
        MediaProvider::MAL => MalClient::get_manga_list(&params.params).await,
        MediaProvider::ANILIST => AniListClient::get_manga_list(&params.params).await,
    };

    match manga_list {
        Err(e) => fail(None, "FETCH_MANGA_FAILED", &e.to_string()),
        Ok(v) => {
            debug!(
                "found {:?} mangas for {:?} from {:?}",
                v.data.len(),
                &params,
                provider
            );
            success(v)
        }
    }
}
