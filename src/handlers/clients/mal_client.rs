use std::collections::HashMap;

use anyhow::{Result, bail};
use reqwest::{Client, StatusCode, Url};
use serde_json::Value;

use crate::{
    db::user_repo::fetch_user_by_id,
    handlers::clients::{MediaClient, MedialClientParams},
    models::media::MediaItem,
};

const DEFAULT_MAL_BASE_URL: &str = "https://api.myanimelist.net/v2";
const DEFAULT_USER_AGENT: &str = "Komorebi-App/1.0";
const MAL_ANIME_FIELDS: &str = "synopsis,media_type,my_list_status,rating,mean,num_episodes,popularity,alternative_titles,genres";
const MAL_MANGA_FIELDS: &str = "synopsis,media_type,my_list_status,mean,num_chapters,num_volumes,popularity,alternative_titles,genres";

#[derive(Default)]
pub struct MalClient {}

impl MediaClient for MalClient {
    async fn get_anime_list(params: &MedialClientParams) -> Result<Vec<MediaItem>> {
        let client = Client::new();
        let mut url = Url::parse(DEFAULT_MAL_BASE_URL)?.join("users")?;
        let username = match params.user_id {
            Some(id) => fetch_user_by_id(id).await?.username,
            None => "@me".to_string(),
        };
        url = url.join(&username)?.join("/animelist")?;

        let mut query: HashMap<&str, String> = HashMap::new();
        if let Some(status) = &params.status {
            query.insert("status", status.clone());
        }
        if let Some(sort) = &params.sort {
            query.insert("sort", sort.clone());
        }
        if let Some(limit) = &params.limit {
            query.insert("limit", limit.to_string());
        }
        if let Some(offset) = &params.offset {
            query.insert("offset", offset.to_string());
        }

        let resp = client.get(url).query(&query);

        if resp.status() != StatusCode::OK {
            bail!("failed due to status code {:?}", resp.status())
        }

        let _res = resp.json::<Value>().await?;
        bail!("internal error occurred")
    }

    async fn get_manga_list(_params: &MedialClientParams) -> Result<Vec<MediaItem>> {
        todo!()
    }
}
