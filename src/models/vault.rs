pub use super::_entities::vault::{ActiveModel, Entity, Model};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{entity::prelude::*, ActiveValue};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub type VaultItem = Model;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, DeriveActiveEnum, EnumIter, TS,
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
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
