use loco_rs::Result;
use sea_orm::{
    IntoActiveModel, TransactionTrait,
    prelude::{ActiveModelTrait, DbConn, EntityTrait},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::models::{
    audio_tracks::{self, AudioTrack},
    subtitle_fonts::{self, SubtitleFont},
    vault_metadata::VaultMetadata,
    vault_sub_item::VaultSubItem,
    video_chapters::{self, VideoChapter},
    video_subtitles::{self, VideoSubtitle},
};

#[derive(Serialize, Deserialize, Default, Clone, TS)]
#[ts(export)]
pub struct VaultSubItemDto {
    #[serde(flatten)]
    pub sub_item: VaultSubItem,
    pub metadata: Option<VaultMetadataDto>,
}

#[derive(Serialize, Deserialize, Default, Clone, TS)]
#[ts(export)]
pub struct VaultMetadataDto {
    #[serde(flatten)]
    pub vault_metadata: VaultMetadata,

    pub audio_tracks: Vec<AudioTrack>,

    pub video_subtitles: Vec<VideoSubtitle>,

    pub video_chapters: Vec<VideoChapter>,

    pub subtitle_fonts: Vec<SubtitleFont>,
}

macro_rules! add_metadata_id {
    ($id:expr, $( $item:expr ),+ $(,)?) => {(
        $(
            $item.into_iter().map(|mut v| {
                v.id = Uuid::now_v7();
                v.metadata_id = $id;
                v.into_active_model()
            }).collect::<Vec<_>>(),
        )*
    )};
}

macro_rules! insert_if_populated {
    ($entity:ty, $data:expr, $txn:expr) => {
        if !$data.is_empty() {
            <$entity>::insert_many($data).exec($txn).await?;
        }
    };
}

impl VaultMetadataDto {
    pub async fn save_vault_metadata_dto(self, db: &DbConn) -> Result<()> {
        // Assign every one ids
        let mut metadata = self.vault_metadata;
        let metadata_id = Uuid::now_v7();

        metadata.id = metadata_id;

        let (audio_tracks, subtitle_fonts, video_chapters, video_subtitles) = add_metadata_id!(
            metadata_id,
            self.audio_tracks,
            self.subtitle_fonts,
            self.video_chapters,
            self.video_subtitles,
        );

        {
            let txn = db.begin().await?;

            metadata.into_active_model().insert(&txn).await?;

            insert_if_populated!(audio_tracks::Entity, audio_tracks, &txn);
            insert_if_populated!(subtitle_fonts::Entity, subtitle_fonts, &txn);
            insert_if_populated!(video_chapters::Entity, video_chapters, &txn);
            insert_if_populated!(video_subtitles::Entity, video_subtitles, &txn);

            txn.commit().await?;
        }
        Ok(())
    }
}
