// pub mod image;
// pub mod novel;
pub mod processor;
pub mod video;

use std::{path::PathBuf, sync::Arc};

use loco_rs::Result;
use phf::{Map, phf_map};

use crate::{
    downloaders::manager::DownloadManager,
    models::{media::MediaType, vault_sub_item::VaultSubItem},
};

pub trait PostProcessor {
    fn post_process(
        file_path: PathBuf,
        manager: Arc<DownloadManager>,
        item: VaultSubItem,
    ) -> impl Future<Output = Result<()>>;
}

const EXT_VS_TYPE: Map<&str, MediaType> = phf_map! {
    "mkv" | "mp4" | "av1" => MediaType::Anime,
    "png" | "pdf" | "webp" => MediaType::Manga,
};
