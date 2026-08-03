pub mod anilist_client;
pub mod mal_client;
pub mod mal_models;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::media::PaginatedResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct MedialClientParams {
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
}

pub trait MediaClient {
    fn get_anime_list(
        params: &MedialClientParams,
    ) -> impl Future<Output = Result<PaginatedResponse>>;

    fn get_manga_list(
        params: &MedialClientParams,
    ) -> impl Future<Output = Result<PaginatedResponse>>;
}
