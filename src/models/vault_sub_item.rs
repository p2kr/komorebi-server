use chrono::Utc;
use educe::Educe;
use loco_rs::prelude::async_trait;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, DbConn};

use crate::models::vault::VaultStatus;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dtos::media::MediaType;

pub type VaultSubItem = Model;

#[sea_orm::model]
#[derive(Clone, Debug, Educe, PartialEq, DeriveEntityModel, Serialize, Deserialize, TS)]
#[educe(Default)]
#[sea_orm(table_name = "vault_sub_item")]
#[ts(export, rename = "VaultSubItem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub vault_id: Uuid,
    /// `vault/<vault id>/<file.mkv>`
    pub source_path: String,
    /// `vault/<vault id>/encoded/<sub id>/metadata.json`
    pub dest_path: Option<String>,
    pub media_type: MediaType,
    pub media_id: Option<String>,
    pub title: String,
    pub raw_title: String,
    pub season: Option<String>,
    pub episode: Option<String>,

    #[educe(Default = VaultStatus::PROCESSING)]
    pub status: VaultStatus,

    pub total_bytes: i64,
    pub progress: f64,
    /// bytes/second
    pub speed_bps: i64,
    pub eta_seconds: Option<i64>,
    pub error_msg: Option<String>,

    #[educe(Default = Utc::now())]
    pub created_at: DateTimeUtc,

    #[educe(Default = Utc::now())]
    pub updated_at: DateTimeUtc,

    #[sea_orm(belongs_to, from = "vault_id", to = "id")]
    #[ts(as = "super::vault::Model")]
    pub vault_item: BelongsTo<super::vault::Entity>,

    #[sea_orm(has_one)]
    #[ts(as = "Option<super::vault_metadata::Model>")]
    pub metadata: HasOne<super::vault_metadata::Entity>,
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

impl Model {
    pub fn to_active_model_and_update_progress(&self) -> ActiveModel {
        ActiveModel {
            id: Set(self.id),
            total_bytes: Set(self.total_bytes),
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

impl ActiveModel {}

impl Entity {
    pub async fn find_by_vault_id(db: &DbConn, vault_id: Uuid) -> Vec<Model> {
        Self::find()
            .filter(Column::VaultId.eq(vault_id))
            .all(db)
            .await
            .unwrap_or_default()
    }

    pub async fn find_incomplete_by_vault_id(db: &DbConn, vault_id: Uuid) -> Vec<Model> {
        Self::find()
            .filter(Column::VaultId.eq(vault_id))
            .filter(Column::Status.ne(VaultStatus::READY))
            .all(db)
            .await
            .unwrap_or_default()
    }

    pub async fn purge_vault_sub_items(db: &DbConn, vault_id: Uuid) -> Result<()> {
        Self::delete_many()
            .filter(Column::VaultId.eq(vault_id))
            .exec(db)
            .await?;

        Ok(())
    }
}
