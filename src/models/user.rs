use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::media::MediaProvider;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
    pub provider: MediaProvider,

    #[serde(skip_serializing)]
    pub access_token: Option<String>,

    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: Uuid::now_v7(),
            username: Default::default(),
            avatar_url: Default::default(),
            provider: Default::default(),
            access_token: Default::default(),
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
        let mut v = Self::default();
        v.username = username;
        v.avatar_url = avatar_url;
        v.provider = provider.unwrap_or_default();
        v.access_token = access_token;
        return v;
    }
}
