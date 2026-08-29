pub mod anilist_client;
pub mod anilist_models;
pub mod mal_client;
pub mod mal_models;

use loco_rs::Result;
use loco_rs::prelude::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    adapters::{anilist_client::AniListClient, mal_client::MalClient},
    models::{
        media::{MediaProvider, PaginatedResponse},
        users::User,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaClientParams {
    pub user_id: Uuid,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

pub type MedialClientParams = MediaClientParams;

impl Default for MediaClientParams {
    fn default() -> Self {
        Self {
            user_id: Uuid::default(),
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

    async fn get_anime_list(&self, params: &MediaClientParams) -> Result<PaginatedResponse>;

    async fn get_manga_list(&self, params: &MediaClientParams) -> Result<PaginatedResponse>;

    async fn validate_new_user(&self, access_token: &str) -> Result<User>;

    async fn exchange_oauth_token(&self, code: &str, code_verifier: &str) -> Result<String>;
}

impl MediaProvider {
    pub fn new_client(&self, client: &Client, user: &User) -> Box<dyn MediaClient + Send + Sync> {
        let client: Box<dyn MediaClient + Send + Sync> = match self {
            MediaProvider::MAL => Box::new(MalClient::new(client, user)),
            MediaProvider::ANILIST => Box::new(AniListClient::new(client, user)),
        };
        client
    }
}
