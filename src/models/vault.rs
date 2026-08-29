pub use super::_entities::vault::{ActiveModel, Entity, Model};
use chrono::Utc;
use loco_rs::prelude::async_trait;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, entity::prelude::*};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub type VaultItem = Model;

impl Default for VaultItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            user_id: Default::default(),
            destination_path: Default::default(),
            media_type: Default::default(),
            media_id: Default::default(),
            title: Default::default(),
            raw_title: Default::default(),
            season: Some("?".into()),
            episode: Some("?".into()),
            source_url: Default::default(),
            download_type: VaultDownloadType::MAGNET,
            status: VaultItemStatus::PENDING,
            total_bytes: 0,
            downloaded_bytes: 0,
            progress: 0.0,
            speed_bps: 0,
            eta_seconds: None,
            temp_path: Default::default(),
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
    PENDING,
    DOWNLOADING,
    PAUSED,
    COMPLETED,
    FAILED,
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

        self
    }
}

// implement your custom finders, selectors oriented logic here
impl Entity {}
