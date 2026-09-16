use chrono::Utc;
use educe::Educe;
use loco_rs::prelude::async_trait;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use sea_orm::entity::prelude::*;

pub use crate::dtos::{MediaType, VaultDownloadType, VaultStatus};

pub type VaultItem = Model;

#[sea_orm::model]
#[derive(Clone, Debug, Educe, PartialEq, DeriveEntityModel, Serialize, Deserialize, TS)]
#[educe(Default)]
#[sea_orm(table_name = "vault")]
#[ts(export, rename = "VaultItem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    /// `vault/<id>`
    pub dest_path: String,
    pub media_type: MediaType,
    pub media_id: Option<String>,
    pub title: String,
    pub raw_title: String,
    pub source_url: String,
    pub download_type: VaultDownloadType,
    pub status: VaultStatus,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    #[sea_orm(column_type = "Float")]
    pub progress: f64,
    pub speed_bps: i64,
    pub eta_seconds: Option<i64>,
    pub error_msg: Option<String>,

    #[educe(Default = Utc::now())]
    pub created_at: DateTimeUtc,

    #[educe(Default = Utc::now())]
    pub updated_at: DateTimeUtc,

    #[sea_orm(has_many)]
    #[ts(as = "Vec<super::vault_sub_item::Model>")]
    pub sub_items: HasMany<super::vault_sub_item::Entity>,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    #[ts(as = "super::users::Model")]
    pub user: BelongsTo<super::users::Entity>,
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _: &C, insert: bool) -> std::result::Result<Self, DbErr>
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

        if insert {
            self.created_at = ActiveValue::Set(Utc::now());
        }

        self.updated_at = ActiveValue::Set(Utc::now());

        Ok(self)
    }
}

// implement your read-oriented logic here
impl Model {
    pub fn to_active_model_and_update_progress(&self) -> ActiveModel {
        ActiveModel {
            id: Set(self.id),
            total_bytes: Set(self.total_bytes),
            downloaded_bytes: Set(self.downloaded_bytes),
            progress: Set(self.progress),
            speed_bps: Set(self.speed_bps),
            eta_seconds: Set(self.eta_seconds),
            status: Set(self.status),
            error_msg: Set(self.error_msg.clone()),
            ..Default::default()
        }
    }

    pub fn to_active_model_and_update_status(
        &self,
        new_status: VaultStatus,
        error_msg: Option<String>,
    ) -> ActiveModel {
        ActiveModel {
            id: Set(self.id),
            status: Set(new_status),
            error_msg: Set(error_msg),
            ..Default::default()
        }
    }
}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
