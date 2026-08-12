use async_trait::async_trait;
use serde_json::Value;
use tracing::debug;

use crate::{
    adapters::{
        MediaClient, MediaClientParams,
        mal_models::{MalResponse, MalStatus},
    },
    core::{AppError, ENV_CONFIGS},
    models::{
        media::{MediaProvider, PaginatedResponse},
        user::User,
    },
};

const DEFAULT_MAL_BASE_URL: &str = "https://api.myanimelist.net/v2";
const HEADER_NAME: &str = "X-MAL-CLIENT-ID";
const MAL_ANIME_FIELDS: &str = "synopsis,media_type,my_list_status,rating,mean,num_episodes,popularity,alternative_titles,genres";
const MAL_MANGA_FIELDS: &str = "synopsis,media_type,my_list_status,mean,num_chapters,num_volumes,popularity,alternative_titles,genres";
const USER_INFO_URL: &str = "https://api.myanimelist.net/v2/users/@me?fields=id,name,picture";

pub struct MalClient {
    client: reqwest::Client,
    user: User,
}

impl MalClient {
    pub async fn fetch_list(
        &self,
        params: &MediaClientParams,
        is_manga: bool,
    ) -> Result<PaginatedResponse, AppError> {
        let username = self.user.username.as_str();

        let (endpoint, fields) = if is_manga {
            ("mangalist", MAL_MANGA_FIELDS)
        } else {
            ("animelist", MAL_ANIME_FIELDS)
        };

        let base_url = format!("{DEFAULT_MAL_BASE_URL}/users/{username}/{endpoint}");

        let norm_status = params
            .status
            .as_deref()
            .and_then(|s| MalStatus::try_from((s, is_manga)).ok());

        let mut req_builder = self.client.get(&base_url).query(&[("fields", fields)]);

        if let Some(status) = norm_status {
            req_builder = req_builder.query(&[("status", status)]);
        }
        if let Some(sort) = &params.sort {
            req_builder = req_builder.query(&[("sort", sort)]);
        }
        req_builder = req_builder.query(&[
            ("limit", params.limit.unwrap_or(50)),
            ("offset", params.offset.unwrap_or(0)),
        ]);

        if let Some(access_token) = self.user.access_token.as_deref() {
            req_builder = req_builder.bearer_auth(access_token);
        }
        if let Some(client_id) = ENV_CONFIGS.mal_client_id.as_deref() {
            req_builder = req_builder.header(HEADER_NAME, client_id);
        }

        let resp = req_builder.send().await?;

        debug!("response received from {:?}", resp.url());

        if !resp.status().is_success() {
            return Err(AppError::UpstreamApi {
                provider: "MAL".to_string(),
                message: format!("HTTP status {}", resp.status()),
            });
        }

        let res = resp.json::<MalResponse>().await?;

        Ok(res.into())
    }
}

#[async_trait]
impl MediaClient for MalClient {
    fn new(client: &reqwest::Client, user: &User) -> Self {
        Self {
            client: client.clone(),
            user: user.clone(),
        }
    }

    async fn get_anime_list(
        &self,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        Self::fetch_list(self, params, false).await
    }

    async fn get_manga_list(
        &self,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        Self::fetch_list(self, params, true).await
    }

    async fn validate_new_user(&self, access_token: &str) -> Result<User, AppError> {
        let resp = self
            .client
            .get(USER_INFO_URL)
            .header(
                HEADER_NAME,
                ENV_CONFIGS.mal_client_id.as_deref().unwrap_or_default(),
            )
            .bearer_auth(access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::UpstreamApi {
                provider: "MAL".to_string(),
                message: format!("HTTP status {}", resp.status()),
            });
        }

        let user: Value = resp.json().await?;

        let username = user["name"]
            .as_str()
            .ok_or(AppError::UpstreamApi {
                provider: "MAL".to_string(),
                message: "error getting username".to_string(),
            })?
            .to_string();

        let new_user = User {
            username,
            provider_id: user["id"].as_str().map(|id| id.to_string()),
            avatar_url: user["picture"].as_str().map(|url| url.to_string()),
            provider: MediaProvider::MAL,
            access_token: Some(access_token.to_string()),
            is_sandbox: false,
            ..Default::default()
        };

        Ok(new_user)
    }
}
