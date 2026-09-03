// pub mod image;
// pub mod novel;
pub mod processor;
pub mod video;

use std::path::PathBuf;

use loco_rs::Result;
use phf::{Map, phf_map};

use crate::{
    downloaders::manager::DownloadManager,
    models::{media::MediaType, vault::VaultItem},
};

pub trait PostProcessor {
    fn resolve_file_path(folder: &str) -> impl Future<Output = Result<(PathBuf, MediaType)>>;
    fn post_process(
        file_path: PathBuf,
        manager: &DownloadManager,
        item: VaultItem,
    ) -> impl Future<Output = Result<()>>;
}

const EXT_VS_TYPE: Map<&str, MediaType> = phf_map! {
    "mkv" | "mp4" | "av1" => MediaType::Anime,
    "png" | "pdf" | "webp" => MediaType::Manga,
};
