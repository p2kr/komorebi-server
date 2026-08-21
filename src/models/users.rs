use async_trait::async_trait;
use chrono::Utc;
use loco_rs::{hash, prelude::*};
use migration::OnConflict;
use uuid::Uuid;

use crate::models::{media::MediaProvider, users::users::Column};

use super::_entities::users::ActiveModel;
pub use super::_entities::users::{self, Entity, Model};

pub type User = Model;

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let needs_id = match &self.id {
            ActiveValue::Set(id) | ActiveValue::Unchanged(id) => id.is_nil(),
            ActiveValue::NotSet => true,
        };
        if insert && needs_id {
            self.id = ActiveValue::Set(Uuid::now_v7());
        }

        let is_sandbox = match &self.access_token {
            ActiveValue::Set(Some(token)) | ActiveValue::Unchanged(Some(token)) => token.is_empty(),
            _ => true,
        };
        self.is_sandbox = ActiveValue::Set(is_sandbox);

        if needs_id {
            self.created_at = ActiveValue::Set(Utc::now().fixed_offset());
        }

        self.updated_at = ActiveValue::Set(Utc::now().fixed_offset());

        Ok(self)
    }
}

impl Model {
    pub async fn save_user(db: &DatabaseConnection, user: Self) -> ModelResult<Self> {
        let mut active_model = user.into_active_model();
        active_model = active_model.before_save(db, true).await?;
        let new_user = Entity::insert(active_model)
            .on_conflict(
                OnConflict::columns([Column::Username, Column::Provider, Column::IsSandbox])
                    .update_columns([
                        Column::AccessToken,
                        Column::IsSandbox,
                        Column::AvatarUrl,
                        Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec_with_returning(db)
            .await?;

        tracing::debug!("Saved/Updated user {:?}", new_user);
        Ok(new_user)
    }

    pub async fn get_all_users(db: &DatabaseConnection) -> ModelResult<Vec<Self>> {
        let users = Entity::find().all(db).await?;
        Ok(users)
    }

    /// Finds a user by ID
    pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> ModelResult<Self> {
        let user = Entity::find_by_id(id).one(db).await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Finds a user by username and provider
    pub async fn find_by_username_and_provider_and_sandbox(
        db: &DatabaseConnection,
        username: &str,
        provider: MediaProvider,
        is_sandbox: bool,
    ) -> ModelResult<Self> {
        let user = Entity::find()
            .filter(Column::Username.eq(username))
            .filter(Column::Provider.eq(provider))
            .filter(Column::IsSandbox.eq(is_sandbox))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    pub async fn delete_user(db: &DatabaseConnection, id: Uuid) -> ModelResult<()> {
        users::Entity::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    /// Verifies if the provided passcode matches the user's passcode.
    /// Note: An empty passcode (or None) is valid when the stored passcode is empty / None.
    #[must_use]
    pub fn verify_passcode(&self, input_passcode: Option<&str>) -> bool {
        let stored = self.passcode.as_deref().unwrap_or("");
        let input = input_passcode.unwrap_or("");

        if stored.is_empty() {
            return input.is_empty();
        }

        if input.is_empty() {
            return false;
        }

        if stored.starts_with("$argon2") {
            hash::verify_password(input, stored)
        } else {
            stored == input
        }
    }
}
