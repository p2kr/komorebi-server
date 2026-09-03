pub struct Streaming;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use loco_rs::Result;
use tokio::{fs, task::JoinHandle};
use uuid::Uuid;

use crate::{
    downloaders::{manager::DownloadManager, torrent::TorrentDownloader},
    models::{
        media::MediaType,
        vault::{VaultDownloadType, VaultItem, VaultItemStatus},
    },
    streaming::{PostProcessor, video::VideoProcessor},
};

impl Streaming {
    pub async fn resolve_file_path(folder: &str) -> Result<(PathBuf, MediaType)> {
        VideoProcessor::resolve_file_path(folder).await
    }

    pub fn post_process(manager: Arc<DownloadManager>, item: VaultItem) -> JoinHandle<()> {
        tokio::spawn(async move {
            match Self::resolve_file_path(&item.destination_path).await {
                Ok((file_path, media_type)) => match media_type {
                    MediaType::Anime => {
                        let (final_status, error_msg) = match VideoProcessor::post_process(
                            file_path.clone(),
                            &manager,
                            item.clone(),
                        )
                        .await
                        {
                            Ok(_) => (VaultItemStatus::READY, None),
                            Err(e) => {
                                tracing::error!(error_msg=%e, "video post processing failed");
                                (VaultItemStatus::FAILED, Some(e.to_string()))
                            }
                        };

                        if let Some(mut item) = manager.active_items.get_mut(&item.id) {
                            if final_status == VaultItemStatus::READY {
                                item.progress = 100.0;
                            }
                            item.status = final_status.clone();
                            item.error_msg = error_msg;
                        }
                        if final_status == VaultItemStatus::READY {
                            // delete/truncate original file.
                            Self::remove_original(
                                &file_path,
                                &manager,
                                &item.download_type,
                                item.id,
                            )
                            .await;
                        }
                    }
                    _ => {
                        tracing::warn!("not yet implemented");
                        if let Some(mut item) = manager.active_items.get_mut(&item.id) {
                            item.status = VaultItemStatus::CANCELLED;
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Error resolving path {}", e);
                    if let Some(mut item) = manager.active_items.get_mut(&item.id) {
                        item.status = VaultItemStatus::FAILED;
                    }
                }
            };
        })
    }

    pub async fn remove_original(
        file_path: &Path,
        manager: &DownloadManager,
        item_type: &VaultDownloadType,
        id: Uuid,
    ) {
        //  Remove file lock from torrent downloader
        if let Some(engine) = manager.get_engine(item_type)
            && let Some(td) = engine.as_any().downcast_ref::<TorrentDownloader>()
            && let Err(e) = td.remove_handle(id).await
        {
            tracing::error!(error=%e,"failed to delete torrent handle");
            return;
        }

        if let Err(e) = fs::remove_file(file_path).await {
            tracing::warn!(error=%e, "failed to delete original file");

            // Open with write access and automatically truncate to 0 bytes
            let truncate_result = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&file_path)
                .await;

            if let Err(e) = truncate_result {
                tracing::error!(error=%e, "failed to truncate original file");
            }
        }
    }
}
