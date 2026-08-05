use anyhow::{Result, bail};
use reqwest::Client;
use serde_json::json;
use tracing::debug;

use crate::{
    db::user_repo::fetch_user_by_id,
    handlers::clients::{MediaClient, MedialClientParams, anilist_models::AniListResponse},
    models::{configs::ENV_CONFIGS, media::PaginatedResponse},
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
    async fn fetch_list(
        params: &MedialClientParams,
        media_type: &str,
    ) -> Result<PaginatedResponse> {
        let user = fetch_user_by_id(params.user_id).await?;
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

        let client = Client::new();
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
            bail!("failed due to status code {:?}", resp.status());
        }

        let res = resp.json::<AniListResponse>().await?;

        if let Some(ref errors) = res.errors {
            if let Some(err) = errors.first() {
                bail!("AniList GraphQL error: {}", err.message);
            }
        }

        Ok(res.into())
    }
}

impl MediaClient for AniListClient {
    async fn get_anime_list(params: &MedialClientParams) -> Result<PaginatedResponse> {
        Self::fetch_list(params, "ANIME").await
    }

    async fn get_manga_list(params: &MedialClientParams) -> Result<PaginatedResponse> {
        Self::fetch_list(params, "MANGA").await
    }
}
