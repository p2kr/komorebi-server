use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{media::MediaProvider, user::User};

#[derive(Clone)]
pub struct UserRepo {
    db: SqlitePool,
}

impl UserRepo {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Inserts a new user or updates existing fields on conflict of (username, provider).
    /// Generates UUID for new users, resolves `is_sandbox` from `access_token`,
    /// and lets SQLite handle `created_at` & `updated_at` defaults & triggers automatically.
    pub async fn save_user(
        &self,
        username: String,
        avatar_url: Option<String>,
        provider: MediaProvider,
        access_token: Option<String>,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::now_v7();
        let is_sandbox = access_token.is_none();

        sqlx::query_as!(
            User,
            r#"INSERT INTO users (id, username, avatar_url, provider, is_sandbox, access_token)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT(username, provider) DO UPDATE SET
                   avatar_url = excluded.avatar_url,
                   is_sandbox = excluded.is_sandbox,
                   access_token = excluded.access_token
               RETURNING
                   id as "id: Uuid",
                   username,
                   avatar_url,
                   provider as "provider: MediaProvider",
                   is_sandbox,
                   access_token,
                   created_at,
                   updated_at"#,
            id,
            username,
            avatar_url,
            provider as MediaProvider,
            is_sandbox,
            access_token
        )
        .fetch_one(&self.db)
        .await
    }

    pub async fn fetch_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT
                id as "id: Uuid",
                username,
                avatar_url,
                provider as "provider: MediaProvider",
                is_sandbox,
                access_token,
                created_at,
                updated_at
               FROM users WHERE id = $1"#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
    }

    pub async fn fetch_user_by_username(
        &self,
        username: String,
        provider: MediaProvider,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT
                id as "id: Uuid",
                username,
                avatar_url,
                provider as "provider: MediaProvider",
                is_sandbox,
                access_token,
                created_at,
                updated_at
               FROM users WHERE username = $1 AND provider = $2"#,
            username,
            provider as MediaProvider
        )
        .fetch_optional(&self.db)
        .await
    }

    pub async fn fetch_all_users(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY updated_at desc")
            .fetch_all(&self.db)
            .await
    }

    pub async fn delete_user(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
