use chrono::Utc;
use educe::Educe;
use loco_rs::prelude::async_trait;
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub type VideoChapter = Model;

#[sea_orm::model]
#[derive(Clone, Debug, Educe, PartialEq, DeriveEntityModel, Serialize, Deserialize, TS)]
#[educe(Default)]
#[sea_orm(table_name = "video_chapters")]
#[ts(export, rename = "VideoChapter")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Id of chapter in probe result
    pub chapter_id: i64,
    pub metadata_id: Uuid,
    pub title: String,
    #[sea_orm(column_type = "Float")]
    pub start_time: f64,
    #[sea_orm(column_type = "Float")]
    pub end_time: f64,

    #[educe(Default = Utc::now())]
    pub created_at: DateTimeUtc,

    #[educe(Default = Utc::now())]
    pub updated_at: DateTimeUtc,

    #[sea_orm(belongs_to, from = "metadata_id", to = "id")]
    #[ts(as = "super::vault_metadata::Model")]
    pub metadata: BelongsTo<super::vault_metadata::Entity>,
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            if self.id.as_ref().is_nil() {
                self.id = ActiveValue::Set(Uuid::now_v7());
            }
            self.created_at = ActiveValue::Set(Utc::now());
        }
        self.updated_at = ActiveValue::Set(Utc::now());
        Ok(self)
    }
}
