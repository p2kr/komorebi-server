use reqwest::Client;
use serde_json::json;
use tracing::debug;

use crate::{
    adapters::{MediaClient, MediaClientParams, anilist_models::AniListResponse},
    core::{AppError, ENV_CONFIGS},
    models::{media::PaginatedResponse, user::User},
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

#[derive(Default)]
pub struct AniListClient {}

impl AniListClient {
    pub async fn fetch_list(
        client: &Client,
        user: &User,
        params: &MediaClientParams,
        media_type: &str,
    ) -> Result<PaginatedResponse, AppError> {
        let username = user.username.as_str();

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

        debug!("anilist payload {:?}", payload);

        let mut req_builder = client.post(ANILIST_GRAPHQL_URL).json(&payload);

        if let Some(access_token) = user.access_token.as_deref() {
            req_builder = req_builder.bearer_auth(access_token);
        }
        if let Some(client_id) = ENV_CONFIGS.anilist_client_id.as_ref() {
            req_builder = req_builder.query(&[("client_id", &client_id)]);
        }

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

impl MediaClient for AniListClient {
    async fn get_anime_list(
        client: &Client,
        user: &User,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        Self::fetch_list(client, user, params, "ANIME").await
    }

    async fn get_manga_list(
        client: &Client,
        user: &User,
        params: &MediaClientParams,
    ) -> Result<PaginatedResponse, AppError> {
        Self::fetch_list(client, user, params, "MANGA").await
    }
}
