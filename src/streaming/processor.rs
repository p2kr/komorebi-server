use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::dtos::{MediaType, events::AppEvent, vault::VaultSubItemDto};
use cached::cached;
use dashmap::DashMap;
use futures::{StreamExt, stream};
use loco_rs::Result;
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QueryFilter,
    TransactionTrait,
};
use tokio::{
    fs,
    sync::{Notify, broadcast::Sender, futures::Notified},
    task::{self},
};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    core::constants::ENCODED_LOC,
    crawlers::crawler_engine::CrawlerEngine,
    downloaders::{manager::DownloadManager, torrent::TorrentDownloader},
    models::{
        vault::{VaultDownloadType, VaultItem, VaultStatus},
        vault_sub_item::{
            Column as VaultSubItemColumn, Entity as VaultSubItemEntity, VaultSubItem,
        },
    },
    streaming::{EXT_VS_TYPE, PostProcessor, video::VideoProcessor},
};

use super::StreamingEvent;

type ActiveItemsMap = DashMap<Uuid, VaultSubItemDto>;

pub struct MediaProcessor {
    pub active_sub_items: Arc<ActiveItemsMap>,
    wakeup: Arc<Notify>,
    pub tx: Sender<AppEvent>,
}

#[cached(max_size = 100, ttl_secs = 15)]
pub async fn cached_resolve_file_paths(folder: &str) -> Vec<(PathBuf, MediaType)> {
    let f = folder.to_string();
    task::spawn_blocking(move || {
        let mut files = vec![];
        for entry in WalkDir::new(f)
            .into_iter()
            .filter_entry(|v| v.file_name() != ENCODED_LOC.as_str())
            .filter_map(|v| v.ok())
        {
            let path = entry.path();
            if path.is_file()
                && let Some(Some(ext)) = path.extension().map(|v| v.to_str())
                && let Some(media_type) = EXT_VS_TYPE.get(ext)
                && let Ok(metadata) = path.metadata()
                // min file size 1 MB to avoid sample/advertisement video/images
                && metadata.len() >= 1_000_000
            {
                files.push((path.to_path_buf(), *media_type));
            }
        }
        files
    })
    .await
    .inspect_err(|e| tracing::error!(folder=%folder, error=%e, "failed to resolve file paths"))
    .unwrap_or_default()
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

impl MediaProcessor {
    pub fn notification(&self) -> Notified<'_> {
        self.wakeup.notified()
    }

    pub fn wake_daemon(&self) {
        self.wakeup.notify_waiters();
    }

    pub async fn start(
        self: Arc<Self>,
        db: &DbConn,
        vault_item: &VaultItem,
        _manager: Arc<DownloadManager>,
    ) {
        match self.create_vault_sub_items(db, vault_item).await {
            Ok(_) => match self.post_process(vault_item).await {
                Ok(_) => tracing::info!("post processed vault items of {}", vault_item.id),
                Err(e) => {
                    tracing::error!(error=%e, "failed to post process vault item {}", vault_item.id)
                }
            },
            Err(e) => {
                tracing::error!(error=%e, "error creating sub items");
                // TODO: Remove directories created.
            }
        }
    }

    async fn get_active_items(&self, db: &DbConn) {
        if let Ok(v) = VaultSubItemEntity::find()
            .filter(VaultSubItemColumn::Status.is_not_in([
                VaultStatus::READY,
                VaultStatus::CANCELLED,
                VaultStatus::FAILED,
            ]))
            .all(db)
            .await
        {
            for item in v {
                self.active_sub_items.insert(item.id, item.into());
            }
        }
    }

    pub async fn new(db: &DbConn, tx: Sender<AppEvent>) -> Arc<Self> {
        let s = Arc::new(Self {
            active_sub_items: Arc::new(DashMap::new()),
            wakeup: Arc::new(Notify::new()),
            tx,
        });
        s.get_active_items(db).await;
        s
    }

    pub async fn create_vault_sub_items(&self, db: &DbConn, item: &VaultItem) -> Result<()> {
        let files = cached_resolve_file_paths(&item.dest_path).await;

        // Remove existing sub items
        VaultSubItemEntity::purge_vault_sub_items(db, item.id).await?;

        let mut insert_items = vec![];

        for (file_path, media_type) in files.iter() {
            // TODO: Parallelize
            let item = item.clone();
            // create sub-item
            if let Some(source_path) = file_path.to_str()
                && let Some(Some(file_name)) = file_path.file_name().map(|v| v.to_str())
            {
                let parsed_results = CrawlerEngine::get_title_parser_result(file_name);
                let title = parsed_results
                    .title
                    .first()
                    .unwrap_or(&file_name.to_string())
                    .to_string();
                let id = Uuid::now_v7();
                let sub_items = VaultSubItem {
                    id,
                    vault_id: item.id,
                    source_path: source_path.to_owned(),
                    media_type: *media_type,
                    title,
                    raw_title: file_name.to_string(),
                    season: parsed_results.season.first().cloned(),
                    episode: parsed_results.episode.first().cloned(),
                    ..Default::default()
                };

                let mut active_model = sub_items.into_active_model();
                // update id and timestamps
                active_model = active_model.before_save(db, true).await?;
                insert_items.push(active_model);
            }
        }

        if !insert_items.is_empty()
            && let Ok(tx) = db.begin().await
        {
            VaultSubItemEntity::insert_many(insert_items)
                .exec(&tx)
                .await?;

            tx.commit().await?;

            // Update active items
            self.get_active_items(db).await;
        }

        Ok(())
    }

    pub async fn post_process(self: Arc<Self>, vault_item: &VaultItem) -> Result<()> {
        if self.active_sub_items.is_empty() {
            tracing::debug!("no sub items to process for {}", vault_item.id);
            StreamingEvent::Complete {
                sub_item_id: Default::default(),
                vault_id: vault_item.id,
                is_last: true,
            }
            .send(&self.tx)
            .ok();
            return Ok(());
        }

        // find all sub items
        let dtos: Vec<VaultSubItemDto> = self
            .active_sub_items
            .iter()
            .filter(|v| v.value().sub_item.vault_id == vault_item.id)
            .map(|entry| entry.value().clone())
            .collect();

        let total_sub_items = dtos.len();

        StreamingEvent::Init {
            vault_id: vault_item.id,
            total: total_sub_items,
        }
        .send(&self.tx)
        .ok();

        let mut tasks = stream::iter(dtos)
            .map(|entry| {
                let bg_self = self.clone();
                let media_type = entry.sub_item.media_type;
                let sub_item_id = entry.sub_item.id;

                async move {
                    let result = match media_type {
                        MediaType::Anime => {
                            VideoProcessor::post_process(bg_self.clone(), &entry).await
                        }
                        // TODO:
                        _ => unimplemented!(),
                    };

                    if let Some(mut entry) = bg_self.active_sub_items.get_mut(&sub_item_id) {
                        handle_result(&mut entry, result).await;
                    };

                    sub_item_id
                }
            })
            .buffer_unordered(3); // Max three ffmpeg process

        while let Some(result) = tasks.next().await {
            StreamingEvent::Complete {
                sub_item_id: result,
                vault_id: vault_item.id,
                is_last: false,
            }
            .send(&self.tx)
            .ok();
        }

        StreamingEvent::Complete {
            sub_item_id: Default::default(),
            vault_id: vault_item.id,
            is_last: true,
        }
        .send(&self.tx)
        .ok();

        Ok(())
    }
}

async fn handle_result(dto: &mut VaultSubItemDto, result: Result<()>) {
    if let Err(e) = result {
        dto.sub_item.status = VaultStatus::FAILED;
        dto.sub_item.error_msg = Some(e.to_string());
    } else {
        dto.sub_item.status = VaultStatus::READY;
        dto.sub_item.progress = 100.0;
        dto.sub_item.eta_seconds = Some(0);
        dto.sub_item.error_msg = None;
        if let Some(metadata) = &dto.metadata {
            // Update final size
            if let Ok(m) = fs::metadata(&metadata.vault_metadata.file_path).await {
                dto.sub_item.total_bytes = m.len() as i64;
            }
            dto.sub_item.dest_path = Some(metadata.vault_metadata.file_path.clone());
        }
    }
}
