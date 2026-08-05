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
        let mut v = Self::default();
        v.username = username;
        v.avatar_url = avatar_url;
        v.provider = provider.unwrap_or_default();
        v.is_sandbox = access_token.is_none();
        v.access_token = access_token;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_is_sandbox_derived_from_access_token() {
        let user_with_token = User::new(
            "test_user".to_string(),
            None,
            Some(MediaProvider::MAL),
            Some("token123".to_string()),
        );
        assert!(!user_with_token.is_sandbox);

        let user_without_token = User::new(
            "test_user".to_string(),
            None,
            Some(MediaProvider::MAL),
            None,
        );
        assert!(user_without_token.is_sandbox);

        let default_user = User::default();
        assert!(default_user.is_sandbox);
    }
}
