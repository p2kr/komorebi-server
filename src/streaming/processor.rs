use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use cached::concurrent_cached;
use dashmap::DashMap;
use loco_rs::Result;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DbConn, EntityTrait, IntoActiveModel,
    QueryFilter, TransactionTrait,
};
use tokio::{
    fs, spawn,
    task::{self, JoinSet},
};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    core::{
        constants::{ENCODED_LOC, METADATA_LOC},
        is_active_status,
    },
    crawlers::crawler_engine::CrawlerEngine,
    downloaders::{manager::DownloadManager, torrent::TorrentDownloader},
    dtos::VaultMetadata,
    models::{
        media::MediaType,
        vault::{VaultDownloadType, VaultItem, VaultItemStatus},
        vault_sub_item::{
            Column as VaultSubItemColumn, Entity as VaultSubItemEntity, VaultSubItem,
        },
    },
    streaming::{EXT_VS_TYPE, PostProcessor, video::VideoProcessor},
};

type ActiveItemsMap = DashMap<Uuid, VaultSubItem>;

pub struct MediaProcessor {
    pub active_sub_items: Arc<ActiveItemsMap>,
    db: DbConn,
}

#[concurrent_cached(max_size = 100)]
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
                files.push((path.to_path_buf(), media_type.clone()));
            }
        }
        files
    })
    .await
    .inspect_err(|e| tracing::error!(folder=%folder, error=%e, "failed to resolve file paths"))
    .unwrap_or_default()
}

impl MediaProcessor {
    pub async fn start(self: Arc<Self>, vault_item: &VaultItem, manager: Arc<DownloadManager>) {
        match self.create_vault_sub_items(vault_item).await {
            Ok(_) => match self.post_process(manager, vault_item).await {
                Ok(_) => tracing::info!("post processed vault items of {}", vault_item.id),
                Err(e) => {
                    tracing::error!(error=%e, "failed to post process vault item {}", vault_item.id)
                }
            },
            Err(e) => {
                tracing::error!(error=%e, "error creating sub items");
            }
        }
    }

    async fn get_active_items(db: &DbConn) -> ActiveItemsMap {
        let map = DashMap::new();
        if let Ok(v) = VaultSubItemEntity::find()
            .filter(VaultSubItemColumn::Status.is_not_in([
                VaultItemStatus::READY,
                VaultItemStatus::CANCELLED,
                VaultItemStatus::FAILED,
            ]))
            .all(db)
            .await
        {
            for item in v {
                // TODO: Remove after finalizing what constitutes as active.
                if is_active_status(&item.status) {
                    map.insert(item.id, item);
                }
            }
        }

        map
    }

    pub async fn new(db: &DbConn) -> Arc<Self> {
        Arc::new(Self {
            db: db.clone(),
            active_sub_items: Arc::new(Self::get_active_items(db).await),
        })
    }

    pub async fn create_vault_sub_items(&self, item: &VaultItem) -> Result<()> {
        let files = cached_resolve_file_paths(&item.destination_path).await;

        // Remove existing sub items
        VaultSubItemEntity::purge_vault_sub_items(&self.db, item.id).await?;

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
                    media_type: Some(media_type.clone()),
                    title,
                    raw_title: file_name.to_string(),
                    season: parsed_results.season.first().cloned(),
                    episode: parsed_results.episode.first().cloned(),
                    source_url: item.source_url.clone(),
                    download_type: item.download_type.clone(),
                    ..Default::default()
                };

                let mut active_model = sub_items.into_active_model();
                // update id and timestamps
                active_model = active_model.before_save(&self.db, true).await?;
                insert_items.push(active_model);
            }
        }
        if !insert_items.is_empty()
            && let Ok(tx) = self.db.begin().await
        {
            VaultSubItemEntity::insert_many(insert_items)
                .exec(&tx)
                .await?;

            tx.commit().await?;
        }

        Ok(())
    }

    pub async fn post_process(
        self: Arc<Self>,
        manager: Arc<DownloadManager>,
        vault_item: &VaultItem,
    ) -> Result<()> {
        // find all sub items
        let sub_items =
            VaultSubItemEntity::find_incomplete_by_vault_id(&self.db, vault_item.id).await;

        if sub_items.is_empty() {
            return Ok(());
        }

        let mut tasks = JoinSet::new();
        let mut sub_items_map = HashMap::new();

        for item in sub_items.iter() {
            sub_items_map.insert(item.id, item.clone());

            let bg_m = manager.clone();
            let id = item.id;
            let item = item.clone();
            let bg_self = self.clone();
            // post process sub items
            tasks.spawn(async move {
                let media_type = item.media_type.clone();
                let result = match media_type.unwrap_or_default() {
                    MediaType::Anime => VideoProcessor::post_process(bg_self, bg_m, item).await,
                    // TODO:
                    _ => unimplemented!(),
                };

                match result {
                    Ok(v) => (VaultItemStatus::READY, None, id, Some(v)),
                    Err(e) => (VaultItemStatus::FAILED, Some(e.to_string()), id, None),
                }
            });
        }
        let mut progress_count = 0;
        let total_count = sub_items.len();
        let vault_metadata_path = PathBuf::from(&vault_item.destination_path).join(METADATA_LOC);
        let mut meta_data = VaultMetadata {
            id: vault_item.id,
            videos: Vec::new(),
        };
        while let Some(task) = tasks.join_next().await {
            match task {
                Ok((status, err_msg, id, meta)) => {
                    // update sub item in db.
                    if let Some(mut sub_item) = sub_items_map.remove(&id) {
                        if status == VaultItemStatus::READY {
                            sub_item.progress = 100.0;
                        }
                        sub_item.status = status.clone();
                        sub_item.error_msg = err_msg.clone();
                        if let Some((path, _)) = &meta {
                            sub_item.dest_path = Some(path.to_string_lossy().into_owned());
                        }
                        sub_item
                            .into_active_model()
                            .update_status_mut(status.clone(), err_msg)
                            .update(&self.db)
                            .await
                            .ok();

                        if status == VaultItemStatus::READY {
                            self.active_sub_items.remove(&id);
                            progress_count += 1;

                            if let Some(mut item) = manager.active_items.get_mut(&vault_item.id) {
                                item.progress =
                                    (progress_count as f64 / total_count as f64) * 100.0;
                            }
                            if let Some((_, data)) = meta {
                                meta_data.videos.push(data);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error=%e, "error joining task");
                }
            }

            manager.wake_daemon();
        }

        if let Some(mut item) = manager.active_items.get_mut(&vault_item.id) {
            if progress_count == total_count {
                item.progress = 100.0;
                item.status = VaultItemStatus::READY;
                item.error_msg = None;

                // Remove all downloaded files.
                let bg_m = manager.clone();
                let vault_id = vault_item.id;
                let dest_path = item.destination_path.clone();
                let download_type = item.download_type.clone();
                spawn(async move {
                    for (file_path, _) in cached_resolve_file_paths(&dest_path).await.iter() {
                        Self::remove_original(file_path, &bg_m, &download_type, vault_id).await;
                    }
                });
            } else if progress_count == 0 {
                item.progress = 0.0;
                item.status = VaultItemStatus::FAILED;
                item.error_msg = Some("All sub items failed".into());
            } else {
                item.progress = (progress_count as f64 / total_count as f64) * 100.0;
                item.status = VaultItemStatus::PARTIAL;
                item.error_msg = Some(format!(
                    "[{}/{}] Sub items failed",
                    total_count - progress_count,
                    total_count
                ))
            }
        }

        manager.wake_daemon();

        // Save meta data to vault root.
        fs::write(vault_metadata_path, serde_json::to_vec_pretty(&meta_data)?).await?;

        Ok(())
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
