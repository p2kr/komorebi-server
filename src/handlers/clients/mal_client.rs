use anyhow::{Result, bail};
use reqwest::Client;
use tracing::debug;

use crate::{
    db::user_repo::fetch_user_by_id,
    handlers::clients::{
        mal_models::{MalResponse, MalStatus},
        MediaClient, MedialClientParams,
    },
    models::{configs::ENV_CONFIGS, media::PaginatedResponse},
};

const DEFAULT_MAL_BASE_URL: &str = "https://api.myanimelist.net/v2";
const HEADER_NAME: &str = "X-MAL-CLIENT-ID";
const MAL_ANIME_FIELDS: &str = "synopsis,media_type,my_list_status,rating,mean,num_episodes,popularity,alternative_titles,genres";
const MAL_MANGA_FIELDS: &str = "synopsis,media_type,my_list_status,mean,num_chapters,num_volumes,popularity,alternative_titles,genres";

#[derive(Default)]
pub struct MalClient {}

impl MalClient {
    async fn fetch_list(params: &MedialClientParams, is_manga: bool) -> Result<PaginatedResponse> {
        let user = fetch_user_by_id(params.user_id).await?;
        let username = user.username.as_str();

        let (endpoint, fields) = if is_manga {
            ("mangalist", MAL_MANGA_FIELDS)
        } else {
            ("animelist", MAL_ANIME_FIELDS)
        };

        let base_url = format!("{DEFAULT_MAL_BASE_URL}/users/{username}/{endpoint}");
        let client = Client::new();

        let norm_status = params
            .status
            .as_deref()
            .and_then(|s| MalStatus::try_from((s, is_manga)).ok());

        let mut req_builder = client.get(&base_url).query(&[("fields", fields)]);

        if let Some(status) = norm_status {
            req_builder = req_builder.query(&[("status", status)]);
        }
        if let Some(sort) = &params.sort {
            req_builder = req_builder.query(&[("sort", sort)]);
        }
        req_builder = req_builder.query(&[
            ("limit", params.limit.unwrap_or_default()),
            ("offset", params.offset.unwrap_or_default()),
        ]);

        if let Some(access_token) = user.access_token.as_deref() {
            req_builder = req_builder.bearer_auth(access_token);
        }
        if let Some(client_id) = ENV_CONFIGS.mal_client_id.as_deref() {
            req_builder = req_builder.header(HEADER_NAME, client_id);
        }

        let resp = req_builder.send().await?;

        debug!("response received from {:?}", resp.url());

        if !resp.status().is_success() {
            bail!("failed due to status code {:?}", resp.status());
        }

        let res = resp.json::<MalResponse>().await?;

        Ok(res.into())
    }
}

impl MediaClient for MalClient {
    async fn get_anime_list(params: &MedialClientParams) -> Result<PaginatedResponse> {
        Self::fetch_list(params, false).await
    }

    async fn get_manga_list(params: &MedialClientParams) -> Result<PaginatedResponse> {
        Self::fetch_list(params, true).await
    }
}
