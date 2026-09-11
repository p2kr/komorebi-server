use chrono::Utc;
use educe::Educe;
use loco_rs::prelude::async_trait;
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

pub type VaultMetadata = Model;

#[sea_orm::model]
#[derive(Clone, Debug, Educe, PartialEq, DeriveEntityModel, Serialize, Deserialize, TS)]
#[educe(Default)]
#[sea_orm(table_name = "vault_metadata")]
#[ts(export, rename = "VaultMetadata")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub sub_item_id: Uuid,
    pub file_name: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,

    #[educe(Default = Utc::now())]
    pub created_at: DateTimeUtc,

    #[educe(Default = Utc::now())]
    pub updated_at: DateTimeUtc,

    #[sea_orm(belongs_to, from = "sub_item_id", to = "id")]
    #[ts(as = "super::vault_sub_item::Model")]
    pub sub_item: BelongsTo<super::vault_sub_item::Entity>,

    #[sea_orm(has_many)]
    #[ts(as = "Vec<super::audio_tracks::Model>")]
    pub audio_tracks: HasMany<super::audio_tracks::Entity>,

    #[sea_orm(has_many)]
    #[ts(as = "Vec<super::video_subtitles::Model>")]
    pub subtitles: HasMany<super::video_subtitles::Entity>,

    #[sea_orm(has_many)]
    #[ts(as = "Vec<super::video_chapters::Model>")]
    pub chapters: HasMany<super::video_chapters::Entity>,

    #[sea_orm(has_many)]
    #[ts(as = "Vec<super::subtitle_fonts::Model>")]
    pub fonts: HasMany<super::subtitle_fonts::Entity>,
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
