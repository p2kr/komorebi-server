use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::debug;

use crate::{
    adapters::{MediaClient, MediaClientParams, anilist_models::AniListResponse},
    core::{AppError, ENV_CONFIGS},
    models::{
        media::{MediaProvider, PaginatedResponse},
        user::User,
    },
};

const ANILIST_GRAPHQL_URL: &str = "https://graphql.anilist.co";

const MEDIA_LIST_QUERY: &str = r#"
query ($userName: String, $type: MediaType, $status: MediaListStatus, $page: Int, $perPage: Int) {
  Page(page: $page, perPage: $perPage) {
    pageInfo {
      total
      perPage
      currentPage
      lastPage
      hasNextPage
    }
    mediaList(userName: $userName, type: $type, status: $status) {
      id
      status
      score(format: POINT_10_DECIMAL)
      progress
      progressVolumes
      repeat
      notes
      updatedAt
      media {
        id
        idMal
        type
        format
        status
        title {
          romaji
          english
          native
          userPreferred
        }
        coverImage {
          extraLarge
          large
          medium
          color
        }
        description
        meanScore
        popularity
        episodes
        duration
        chapters
        volumes
        genres
        isAdult
      }
    }
  }
}
"#;

const USER_INFO_QUERY: &str = r#"
query {
  Viewer {
    id
    name
    about
    avatar {
      medium
    }
    bannerImage
  }
}
"#;

fn normalize_anilist_status(status: Option<&str>) -> Option<&'static str> {
    let s = status?;
    match s.to_uppercase().as_str() {
        "CURRENT" | "WATCHING" | "READING" => Some("CURRENT"),
        "PLANNING" | "PLAN_TO_WATCH" | "PLAN_TO_READ" => Some("PLANNING"),
        "COMPLETED" => Some("COMPLETED"),
        "DROPPED" => Some("DROPPED"),
        "PAUSED" | "ON_HOLD" => Some("PAUSED"),
        "REPEATING" | "REWATCHING" | "REREADING" => Some("REPEATING"),
        _ => None,
    }
}

pub struct AniListClient {
    client: reqwest::Client,
    user: User,
}

impl AniListClient {
    pub async fn fetch_list(
        &self,
        params: &MediaClientParams,
        media_type: &str,
    ) -> Result<PaginatedResponse, AppError> {
        let username = self.user.username.as_str();

        let raw_page = params.offset.unwrap_or(1);
        let page = if raw_page < 1 { 1 } else { raw_page };
        let per_page = params.limit.unwrap_or_default();
        let norm_status = normalize_anilist_status(params.status.as_deref());

        let mut variables = json!({
            "userName": username,
            "type": media_type,
            "page": page,
            "perPage": per_page
        });

        if let Some(status) = norm_status {
            variables["status"] = json!(status);
        }

        let payload = json!({
            "query": MEDIA_LIST_QUERY,
            "variables": variables
        });

        debug!(
            "sending anilist request for user {:?}",
            variables.get("userName")
        );

        let mut req_builder = self.client.post(ANILIST_GRAPHQL_URL).json(&payload);

        if let Some(access_token) = self.user.access_token.as_deref() {
            req_builder = req_builder.bearer_auth(access_token);
        }

        req_builder = req_builder.query(&[("client_id", ENV_CONFIGS.anilist_client_id.as_str())]);

        let resp = req_builder.send().await?;

        debug!("response received from AniList: {:?}", resp.url());

        if !resp.status().is_success() {
            return Err(AppError::UpstreamApi {
                provider: "ANILIST".to_string(),
                message: format!("HTTP status {}", resp.status()),
            });
        }

        let res = resp.json::<AniListResponse>().await?;

        if let Some(err) = res.errors.as_ref().and_then(|errors| errors.first()) {
            return Err(AppError::UpstreamApi {
                provider: "ANILIST".to_string(),
                message: err.message.clone(),
            });
        }

        Ok(res.into())
    }
}

#[async_trait]
impl MediaClient for AniListClient {
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
        Self::fetch_list(self, params, "ANIME").await
    }

    async fn get_manga_list(
        &self,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        Self::fetch_list(self, params, "MANGA").await
    }

    async fn validate_new_user(&self, access_token: &str) -> Result<User, AppError> {
        let resp = self
            .client
            .post(ANILIST_GRAPHQL_URL)
            .json(&json!({
                "query": USER_INFO_QUERY,
                "client_id": ENV_CONFIGS.anilist_client_id
            }))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::UpstreamApi {
                provider: "ANILIST".to_string(),
                message: format!("HTTP status {}", resp.status()),
            });
        }

        let user: Value = resp.json().await?;

        let username = user["data"]["Viewer"]["name"]
            .as_str()
            .ok_or(AppError::UpstreamApi {
                provider: String::from("ANILIST"),
                message: String::from("unable to get username"),
            })?;
        let provider_id = user["data"]["Viewer"]["id"].as_i64();
        let avatar_url = user["data"]["Viewer"]["avatar"]["medium"].as_str();

        let new_user = User {
            username: username.to_string(),
            provider_id: provider_id.map(|id| id.to_string()),
            avatar_url: avatar_url.map(|url| url.to_string()),
            provider: MediaProvider::ANILIST,
            access_token: Some(access_token.to_string()),
            is_sandbox: false,
            ..Default::default()
        };

        Ok(new_user)
    }

    async fn exchange_oauth_token(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<String, AppError> {
        // return either code or code_verifier, whichever has data; because anilist doesn't need exchange.
        if code.is_empty() && code_verifier.is_empty() {
            return Err(AppError::InvalidParams(
                "code and code_verifier cannot be empty".to_string(),
            ));
        } else {
            if !code.is_empty() {
                return Ok(code.to_string());
            } else {
                return Ok(code_verifier.to_string());
            }
        }
    }
}
