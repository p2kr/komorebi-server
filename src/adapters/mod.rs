pub mod anilist_client;
pub mod anilist_models;
pub mod mal_client;
pub mod mal_models;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    adapters::{anilist_client::AniListClient, mal_client::MalClient},
    core::AppError,
    models::{
        media::{MediaProvider, PaginatedResponse},
        user::User,
    },
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
            limit: Some(50),
            offset: Some(0),
        }
    }
}

#[async_trait]
pub trait MediaClient: Send + Sync {
    fn new(client: &reqwest::Client, user: &User) -> Self
    where
        Self: Sized;

    async fn get_anime_list(
        &self,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError>;

    async fn get_manga_list(
        &self,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError>;

    async fn validate_new_user(&self, access_token: &str) -> Result<User, AppError>;
}

impl MediaProvider {
    pub fn new_client(
        &self,
        client: &reqwest::Client,
        user: &User,
    ) -> Box<dyn MediaClient + Send + Sync> {
        let client: Box<dyn MediaClient + Send + Sync> = match self {
            MediaProvider::MAL => Box::new(MalClient::new(client, user)),
            MediaProvider::ANILIST => Box::new(AniListClient::new(client, user)),
        };
        client
    }
}
