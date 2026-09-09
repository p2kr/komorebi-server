use chrono::Utc;
use loco_rs::prelude::async_trait;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use sea_orm::entity::prelude::*;

use crate::models::media::MediaType;

pub type VaultItem = Model;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, TS)]
#[sea_orm(table_name = "vault")]
#[ts(export, rename = "VaultItem")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    #[sea_orm(unique)]
    /// `vault/<id>`
    pub destination_path: String,
    pub media_type: Option<MediaType>,
    pub media_id: String,
    pub title: String,
    pub source_url: String,
    pub download_type: VaultDownloadType,
    pub status: VaultItemStatus,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    #[sea_orm(column_type = "Float")]
    pub progress: f64,
    pub speed_bps: i64,
    pub eta_seconds: Option<i64>,
    pub error_msg: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,

    #[sea_orm(has_many)]
    #[ts(as = "Vec<super::vault_sub_item::Model>")]
    pub sub_items: HasMany<super::vault_sub_item::Entity>,

    #[sea_orm(belongs_to, from = "user_id", to = "id")]
    #[ts(as = "super::users::Model")]
    pub user: BelongsTo<super::users::Entity>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: Default::default(),
            user_id: Default::default(),
            destination_path: Default::default(),
            media_type: Default::default(),
            media_id: Default::default(),
            title: Default::default(),
            source_url: Default::default(),
            download_type: VaultDownloadType::MAGNET,
            status: VaultItemStatus::PENDING,
            total_bytes: 0,
            downloaded_bytes: 0,
            progress: 0.0,
            speed_bps: 0,
            eta_seconds: None,
            error_msg: None,
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Default,
    DeriveActiveEnum,
    EnumIter,
    TS,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "UPPERCASE"
)]
pub enum VaultDownloadType {
    DIRECT, // HTTP/HTTPS direct download
    #[default]
    MAGNET, // Magnet link
    TFILE,  // Torrent file
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, DeriveActiveEnum, EnumIter, TS,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "UPPERCASE"
)]
pub enum VaultItemStatus {
    #[default]
    /// added to queue, not yet downloading
    PENDING,
    /// actively downloading
    DOWNLOADING,
    /// user paused
    PAUSED,
    /// download bytes done (internal transient — daemon picks this up)
    COMPLETED,
    /// remux/transcode in progress
    PROCESSING,
    /// stream-ready
    READY,
    /// Partial ready
    PARTIAL,
    /// download or post-process error
    FAILED,
    /// user deleted
    CANCELLED,
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
    pub fn update_status(mut self, new_status: VaultItemStatus, error_msg: Option<String>) -> Self {
        self.status = ActiveValue::Set(new_status);
        if let Some(msg) = error_msg {
            self.error_msg = ActiveValue::Set(Some(msg));
        }

        self
    }

    pub fn update_progress_mut(mut self) -> Self {
        self.total_bytes.reset();
        self.downloaded_bytes.reset();
        self.progress.reset();
        self.speed_bps.reset();
        self.eta_seconds.reset();
        self.status.reset();
        self.error_msg.reset();

        self
    }
}

// implement your custom finders, selectors oriented logic here
impl Entity {}
