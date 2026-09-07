use chrono::Utc;
use loco_rs::prelude::async_trait;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, DbConn};

use crate::models::vault::{VaultDownloadType, VaultItemStatus};

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::media::MediaType;

pub type VaultSubItem = Model;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, TS)]
#[sea_orm(table_name = "vault_sub_item")]
#[ts(export, rename = "VaultSubItem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub vault_id: Uuid,
    #[sea_orm(unique)]
    pub source_path: String,
    pub dest_path: Option<String>,
    pub media_type: Option<MediaType>,
    pub media_id: Option<String>,
    pub title: String,
    pub raw_title: String,
    pub season: Option<String>,
    pub episode: Option<String>,
    pub source_url: String,
    pub download_type: VaultDownloadType,
    pub status: VaultItemStatus,
    pub total_bytes: i64,
    pub progress: f64,
    pub speed_bps: i64,
    pub eta_seconds: Option<i64>,
    pub error_msg: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(has_one)]
    #[ts(as = "super::vault::Model")]
    pub vault_item: HasOne<super::vault::Entity>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: Default::default(),
            vault_id: Default::default(),
            source_path: Default::default(),
            media_type: Default::default(),
            media_id: None,
            title: Default::default(),
            raw_title: Default::default(),
            season: Some("?".into()),
            episode: Some("?".into()),
            source_url: Default::default(),
            download_type: VaultDownloadType::MAGNET,
            status: VaultItemStatus::PENDING,
            total_bytes: 0,
            progress: 0.0,
            speed_bps: 0,
            eta_seconds: None,
            dest_path: Default::default(),
            error_msg: None,
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
        }
    }
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
            self.created_at = ActiveValue::Set(Utc::now().fixed_offset());
        }

        self.updated_at = ActiveValue::Set(Utc::now().into());

        Ok(self)
    }
}

// implement your read-oriented logic here
impl Model {}

// implement your write-oriented logic here
impl ActiveModel {
    pub fn update_status_mut(
        mut self,
        new_status: VaultItemStatus,
        error_msg: Option<String>,
    ) -> Self {
        self.status = ActiveValue::Set(new_status);
        if let Some(msg) = error_msg {
            self.error_msg = ActiveValue::Set(Some(msg));
        }

        self
    }

    pub fn update_progress_mut(mut self) -> Self {
        self.total_bytes.reset();
        self.progress.reset();
        self.speed_bps.reset();
        self.eta_seconds.reset();
        self.status.reset();
        self.dest_path.reset();
        self.error_msg.reset();

        self
    }
}

// implement your custom finders, selectors oriented logic here
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
            .filter(Column::Status.ne(VaultItemStatus::READY))
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
