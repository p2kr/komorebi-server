// pub mod image;
// pub mod novel;
pub mod processor;
pub mod video;

use std::sync::Arc;

use loco_rs::Result;
use phf::{Map, phf_map};

use crate::dtos::media::MediaType;
use crate::dtos::vault::VaultMetadataDto;
use crate::{
    downloaders::manager::DownloadManager, models::vault_sub_item::VaultSubItem,
    streaming::processor::MediaProcessor,
};

pub trait PostProcessor {
    fn post_process(
        processor: Arc<MediaProcessor>,
        manager: Arc<DownloadManager>,
        item: VaultSubItem,
    ) -> impl Future<Output = Result<VaultMetadataDto>>;
}

const EXT_VS_TYPE: Map<&str, MediaType> = phf_map! {
    "mkv" | "mp4" | "av1" => MediaType::Anime,
    "png" | "pdf" | "webp" => MediaType::Manga,
};
