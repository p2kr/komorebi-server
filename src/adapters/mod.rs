pub mod anilist_client;
pub mod anilist_models;
pub mod mal_client;
pub mod mal_models;

use serde::{Deserialize, Serialize};
use std::future::Future;
use uuid::Uuid;

use crate::{
    core::AppError,
    models::{media::PaginatedResponse, user::User},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaClientParams {
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

pub type MedialClientParams = MediaClientParams;

impl Default for MediaClientParams {
    fn default() -> Self {
        Self {
            user_id: Default::default(),
            status: None,
            sort: None,
            limit: Some(20),
            offset: Some(0),
        }
    }
}

pub trait MediaClient {
    fn get_anime_list(
        client: &reqwest::Client,
        user: &User,
        params: &MediaClientParams,
    ) -> impl Future<Output = Result<PaginatedResponse, AppError>>;

    fn get_manga_list(
        client: &reqwest::Client,
        user: &User,
        params: &MediaClientParams,
    ) -> impl Future<Output = Result<PaginatedResponse, AppError>>;
}
