use anyhow::{Result, bail};
use reqwest::Client;
use tracing::debug;

use crate::{
    db::user_repo::fetch_user_by_id,
    handlers::clients::{MediaClient, MedialClientParams, mal_models::MalResponse},
    models::{configs::ENV_CONFIGS, media::PaginatedResponse},
};

const DEFAULT_MAL_BASE_URL: &str = "https://api.myanimelist.net/v2";
const HEADER_NAME: &str = "X-MAL-CLIENT-ID";
const MAL_ANIME_FIELDS: &str = "synopsis,media_type,my_list_status,rating,mean,num_episodes,popularity,alternative_titles,genres";
const MAL_MANGA_FIELDS: &str = "synopsis,media_type,my_list_status,mean,num_chapters,num_volumes,popularity,alternative_titles,genres";

#[derive(Default)]
pub struct MalClient {}

impl MediaClient for MalClient {
    async fn get_anime_list(params: &MedialClientParams) -> Result<PaginatedResponse> {
        let user = fetch_user_by_id(params.user_id).await?;
        let username = user.username.as_str();

        let base_url = format!("{DEFAULT_MAL_BASE_URL}/users/{username}/animelist");
        let client = Client::new();
        let mut req_builder = client
            .get(&base_url)
            .query(params)
            .query(&[("fields", MAL_ANIME_FIELDS)]);
        if let Some(access_token) = user.access_token.as_deref() {
            req_builder = req_builder.bearer_auth(access_token);
        }
        req_builder = req_builder.header(HEADER_NAME, ENV_CONFIGS.mal_client_id.as_ref().unwrap());

        let resp = req_builder.send().await?;

        debug!("response received from {:#?}", resp.url());

        if !resp.status().is_success() {
            bail!("failed due to status code {:?}", resp.status());
        }

        let res = resp.json::<MalResponse>().await?;

        Ok(res.into())
    }

    async fn get_manga_list(_params: &MedialClientParams) -> Result<PaginatedResponse> {
        todo!()
    }
}
