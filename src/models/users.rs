use chrono::Utc;
use derive_more::Debug;
use loco_rs::prelude::async_trait;
use loco_rs::{hash, prelude::*};
use migration::OnConflict;
use uuid::Uuid;

use crate::models::media::MediaProvider;

pub type User = Model;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[sea_orm::model]
#[derive(Clone, PartialEq, Debug, DeriveEntityModel, Eq, Serialize, Deserialize, Default, TS)]
#[sea_orm(table_name = "users")]
#[serde(default)]
#[ts(export, rename = "User")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    pub username: String,
    pub provider_id: Option<String>,
    pub avatar_url: Option<String>,
    pub provider: MediaProvider,
    pub is_sandbox: bool,

    #[serde(skip_serializing)]
    #[debug("[REDACTED]")]
    pub access_token: Option<String>,

    #[serde(skip_serializing)]
    #[debug("[REDACTED]")]
    pub passcode: Option<String>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(has_many)]
    #[ts(as = "Vec<super::vault::Model>")]
    pub vault_items: HasMany<super::vault::Entity>,
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _: &C, insert: bool) -> Result<Self, DbErr>
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

        if insert {
            self.created_at = ActiveValue::Set(Utc::now().fixed_offset());
        }

        self.updated_at = ActiveValue::Set(Utc::now().fixed_offset());

        Ok(self)
    }
}

impl Model {
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

impl ActiveModel {
    pub async fn save_user(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self = self.before_save(db, true).await?;
        let new_user = Entity::insert(self)
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
}

impl Entity {
    /// Finds a user by username and provider
    pub async fn find_by_username_and_provider_and_sandbox(
        db: &DatabaseConnection,
        username: &str,
        provider: MediaProvider,
        is_sandbox: bool,
    ) -> ModelResult<Model> {
        let user = Entity::find()
            .filter(Column::Username.eq(username))
            .filter(Column::Provider.eq(provider))
            .filter(Column::IsSandbox.eq(is_sandbox))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }
}
