use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::media::MediaProvider;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub provider: MediaProvider,
    pub is_sandbox: bool,

    #[serde(skip_serializing)]
    pub access_token: Option<String>,

    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for User {
    fn default() -> Self {
        let access_token: Option<String> = None;
        Self {
            id: Uuid::now_v7(),
            username: Default::default(),
            avatar_url: Default::default(),
            provider: Default::default(),
            is_sandbox: access_token.is_none(),
            access_token,
            created_at: Utc::now().timestamp_millis(),
            updated_at: Utc::now().timestamp_millis(),
        }
    }
}

impl User {
    pub fn new(
        username: String,
        avatar_url: Option<String>,
        provider: Option<MediaProvider>,
        access_token: Option<String>,
    ) -> Self {
        let is_sandbox = access_token.is_none();
        Self {
            username,
            avatar_url,
            provider: provider.unwrap_or_default(),
            is_sandbox,
            access_token,
            ..Default::default()
        }
    }
}
