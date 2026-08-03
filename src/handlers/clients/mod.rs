pub mod anilist_client;
pub mod mal_client;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::media::MediaItem;

#[derive(Debug, Serialize, Deserialize)]
pub struct MedialClientParams {
    pub user_id: Option<Uuid>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

pub trait MediaClient {
    async fn get_anime_list(params: &MedialClientParams) -> Result<Vec<MediaItem>>;

    async fn get_manga_list(params: &MedialClientParams) -> Result<Vec<MediaItem>>;
}
