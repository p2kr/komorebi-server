use loco_rs::prelude::async_trait;
use loco_rs::{Error, Result};
use serde_json::{Value, json};
use tracing::debug;

use crate::{
    adapters::{
        MediaClient, MediaClientParams,
        mal_models::{MalResponse, MalStatus},
    },
    core::{ResultExt, constants::DEFAULT_HOSTED_AUTH_PAGE},
    models::{
        media::{MediaProvider, PaginatedResponse},
        users::User,
    },
};

const DEFAULT_MAL_BASE_URL: &str = "https://api.myanimelist.net/v2";
const HEADER_NAME: &str = "X-MAL-CLIENT-ID";
const MAL_ANIME_FIELDS: &str = "synopsis,media_type,my_list_status,rating,mean,num_episodes,popularity,alternative_titles,genres";
const MAL_MANGA_FIELDS: &str = "synopsis,media_type,my_list_status,mean,num_chapters,num_volumes,popularity,alternative_titles,genres";
const USER_INFO_URL: &str = "https://api.myanimelist.net/v2/users/@me?fields=id,name,picture";
const MAL_TOKEN_URL: &str = "https://myanimelist.net/v1/oauth2/token";

pub struct MalClient {
    client: reqwest::Client,
    user: User,
}

impl MalClient {
    pub async fn fetch_list(
        &self,
        params: &MediaClientParams,
        is_manga: bool,
    ) -> Result<PaginatedResponse> {
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

        req_builder = req_builder.header(HEADER_NAME, env!("MAL_CLIENT_ID"));

        let resp = req_builder.send().await.to_loco_err()?;

        debug!("response received from {:?}", resp.url());

        if !resp.status().is_success() {
            return Err(Error::Message(format!(
                "Error fetching users for MAL with status {}",
                resp.status()
            )));
        }

        let res = resp.json::<MalResponse>().await.to_loco_err()?;

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

    async fn get_anime_list(&self, params: &MediaClientParams) -> Result<PaginatedResponse> {
        Self::fetch_list(self, params, false).await
    }

    async fn get_manga_list(&self, params: &MediaClientParams) -> Result<PaginatedResponse> {
        Self::fetch_list(self, params, true).await
    }

    async fn validate_new_user(&self, access_token: &str) -> Result<User> {
        let resp = self
            .client
            .get(USER_INFO_URL)
            .header(HEADER_NAME, env!("MAL_CLIENT_ID"))
            .bearer_auth(access_token)
            .send()
            .await
            .to_loco_err()?;

        let user: Value = resp.json().await.to_loco_err()?;

        let username = user["name"]
            .as_str()
            .ok_or(Error::Message("Error getting username".into()))?
            .into();

        let new_user = User {
            username,
            provider_id: user["id"].as_i64().map(|id| id.to_string()),
            avatar_url: user["picture"].as_str().map(|url| url.into()),
            provider: MediaProvider::MAL,
            access_token: Some(access_token.into()),
            is_sandbox: false,
            ..Default::default()
        };

        Ok(new_user)
    }

    async fn exchange_oauth_token(&self, code: &str, code_verifier: &str) -> Result<String> {
        let redirect_uri = option_env!("HOSTED_AUTH_PAGE").unwrap_or(DEFAULT_HOSTED_AUTH_PAGE);
        let resp = self
            .client
            .post(MAL_TOKEN_URL)
            .header("Access-Control-Allow-Origin", MAL_TOKEN_URL)
            .form(&json!({
                "client_id": env!("MAL_CLIENT_ID"),
                "code": code,
                "code_verifier": code_verifier,
                "grant_type": "authorization_code",
                "redirect_uri": redirect_uri
            }))
            .send()
            .await
            .to_loco_err()?;

        debug!("exchange_oauth_token: resp={:#?}", resp);

        if resp.status().is_success() {
            let data: Value = resp.json().await.to_loco_err()?;
            return Ok(data["access_token"].as_str().unwrap_or_default().into());
        }

        Err(Error::Message(
            "Error exchanging oauth token for MAL".into(),
        ))
    }
}
