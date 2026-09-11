use chrono::Utc;
use educe::Educe;
use loco_rs::prelude::async_trait;
use loco_rs::{hash, prelude::*};
use migration::OnConflict;
use uuid::Uuid;

use crate::dtos::MediaProvider;

pub type User = Model;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[sea_orm::model]
#[derive(Clone, PartialEq, Educe, DeriveEntityModel, Eq, Serialize, Deserialize, TS)]
#[educe(Debug, Default)]
#[sea_orm(table_name = "users")]
#[serde(default)]
#[ts(export, rename = "User")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    #[sea_orm(unique_key = "unique_user")]
    pub username: String,
    pub provider_id: Option<String>,
    pub avatar_url: Option<String>,
    #[sea_orm(unique_key = "unique_user")]
    pub provider: MediaProvider,
    #[sea_orm(unique_key = "unique_user")]
    pub is_sandbox: bool,

    #[serde(skip_serializing)]
    #[educe(Debug(ignore))]
    pub access_token: Option<String>,

    #[serde(skip_serializing)]
    #[educe(Debug(ignore))]
    pub passcode: Option<String>,

    #[educe(Default = Utc::now())]
    pub created_at: DateTimeUtc,

    #[educe(Default = Utc::now())]
    pub updated_at: DateTimeUtc,

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
            self.created_at = ActiveValue::Set(Utc::now());
        }

        self.updated_at = ActiveValue::Set(Utc::now());

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

impl Entity {}
