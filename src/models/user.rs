use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::media::MediaProvider;

#[derive(Serialize, Deserialize, PartialEq, Clone, FromRow)]
#[serde(default)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub provider_id: Option<String>,
    pub avatar_url: Option<String>,
    pub provider: MediaProvider,
    pub is_sandbox: bool,

    #[serde(skip_serializing)]
    pub access_token: Option<String>,

    pub created_at: i64,
    pub updated_at: i64,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("provider_id", &self.provider_id)
            .field("avatar_url", &self.avatar_url)
            .field("provider", &self.provider)
            .field("is_sandbox", &self.is_sandbox)
            .field(
                "access_token",
                &self.access_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: Uuid::now_v7(),
            provider_id: Default::default(),
            username: Default::default(),
            avatar_url: Default::default(),
            provider: Default::default(),
            is_sandbox: true,
            access_token: Default::default(),
            created_at: Utc::now().timestamp_millis(),
            updated_at: Utc::now().timestamp_millis(),
        }
    }
}

impl User {
    pub fn new(
        username: String,
        provider_id: Option<String>,
        avatar_url: Option<String>,
        provider: Option<MediaProvider>,
        access_token: Option<String>,
    ) -> Self {
        let is_sandbox = access_token.is_none();
        Self {
            username,
            provider_id,
            avatar_url,
            provider: provider.unwrap_or_default(),
            is_sandbox,
            access_token,
            ..Default::default()
        }
    }
}
