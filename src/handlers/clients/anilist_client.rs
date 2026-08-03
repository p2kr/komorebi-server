use anyhow::Result;

use crate::{
    handlers::clients::{MediaClient, MedialClientParams},
    models::media::PaginatedResponse,
};

// const DEFAULT_MAL_BASE_URL: &str = "https://api.myanimelist.net/v2";
// const DEFAULT_USER_AGENT: &str = "Komorebi-App/1.0";

#[derive(Default)]
pub struct AnilistClient {}

impl MediaClient for AnilistClient {
    async fn get_anime_list(_params: &MedialClientParams) -> Result<PaginatedResponse> {
        todo!()
    }

    async fn get_manga_list(_params: &MedialClientParams) -> Result<PaginatedResponse> {
        todo!()
    }
}
