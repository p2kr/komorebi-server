use std::{fmt::Display, io::SeekFrom, sync::Arc};

use dashmap::DashMap;
use loco_rs::Result;
use loco_rs::prelude::async_trait;
use reqwest::{Client, StatusCode, Url, header};
use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    core::{ResultExt, vault_path_resolver::get_file_path},
    downloaders::DownloadEngine,
    models::vault::{VaultDownloadType, VaultItem, VaultItemStatus},
};

pub struct DirectDownloader {
    client: Client,
    cancel_tokens: DashMap<Uuid, CancellationToken>,
    active_items: Arc<DashMap<Uuid, VaultItem>>,
}

impl Display for DirectDownloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DirectDownloader[client=reqwest::Client, cancel_tokens={}, active_items={}]",
            self.cancel_tokens.len(),
            self.active_items.len()
        )
    }
}

impl DirectDownloader {
    pub async fn new(client: Client, active_items: Arc<DashMap<Uuid, VaultItem>>) -> Arc<Self> {
        Arc::new(Self {
            client,
            cancel_tokens: DashMap::new(),
            active_items: active_items.clone(),
        })
    }
}

#[async_trait]
impl DownloadEngine for DirectDownloader {
    async fn add(&self, vault_item: &VaultItem) -> Result<()> {
        let url = Url::parse(&vault_item.source_url).to_loco_err()?;

        // 1. Ensure the destination directory exists
        fs::create_dir_all(&vault_item.destination_path)
            .await
            .to_loco_err()?;

        // 2. Set up tracking
        let file_path = get_file_path(vault_item);
        let cancel_token = CancellationToken::new();
        self.cancel_tokens
            .insert(vault_item.id, cancel_token.clone());
        self.active_items.insert(vault_item.id, vault_item.clone());

        let client = self.client.clone();
        let active_items = self.active_items.clone();
        let vault_id = vault_item.id;

        // 3. Spawn the background worker
        tokio::spawn(async move {
            let fail_item = |prefix: &str, e: &dyn Display| {
                let msg = format!("{}:{}", prefix, e);
                tracing::error!("{}", msg);
                if let Some(mut item) = active_items.get_mut(&vault_id) {
                    item.status = VaultItemStatus::FAILED;
                    item.error_msg = Some(msg);
                }
            };

            // Determine if we need to resume
            let mut downloaded_bytes = 0;
            let mut req = client.get(url);

            if let Ok(metadata) = fs::metadata(&file_path).await {
                downloaded_bytes = metadata.len();
                req = req.header(header::RANGE, format!("bytes={}-", downloaded_bytes));
            }

            let mut res = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    fail_item("Failed to start direct download", &e);
                    return;
                }
            };

            if res.status() != StatusCode::PARTIAL_CONTENT {
                downloaded_bytes = 0; // The server is sending from the beginning!
            } else {
                // Parse the Content-Range header (e.g., "bytes 1000-9999/10000")
                if let Some(content_range) = res.headers().get(header::CONTENT_RANGE) {
                    if let Ok(range_str) = content_range.to_str() {
                        if let Some(range_info) = range_str.strip_prefix("bytes ") {
                            // Split by '-' and take the first part
                            if let Some(start_str) = range_info.split('-').next() {
                                if let Ok(parsed_start) = start_str.parse::<u64>() {
                                    // The server might have decided to start slightly earlier
                                    // than we requested. We trust the server.
                                    downloaded_bytes = parsed_start;
                                }
                            }
                        }
                    }
                }
            }

            let total_bytes = res.content_length().unwrap_or(0) + downloaded_bytes;

            let mut file = match OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(res.status() != StatusCode::PARTIAL_CONTENT)
                .open(&file_path)
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    let msg = format!("Failed to open file: {}", e);
                    tracing::error!(msg);
                    if let Some(mut item) = active_items.get_mut(&vault_id) {
                        item.status = VaultItemStatus::FAILED;
                        item.error_msg = Some(msg);
                    }
                    return;
                }
            };

            if downloaded_bytes > 0 {
                if let Err(e) = file.seek(SeekFrom::Start(downloaded_bytes)).await {
                    fail_item("Failed to seek file: {}", &e);
                    return;
                }

                // This safely chops off the end of the file if the server told us to start
                // earlier than our physical file length.
                if let Err(e) = file.set_len(downloaded_bytes).await {
                    fail_item("Failed to truncate stale file data: {}", &e);
                    return;
                }
            }

            let mut last_updated = Instant::now();

            // Stream chunks
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        let msg = format!("Download paused/cancelled for vault_id: {}", vault_id);
                        tracing::error!(msg);
                        if let Some(mut item) = active_items.get_mut(&vault_id) {
                            item.status = VaultItemStatus::PAUSED;
                            item.error_msg = Some(msg);
                        }
                        break;
                    }
                    chunk = res.chunk() => {
                        match chunk {
                            Ok(Some(bytes)) => {
                                if let Err(e) = file.write_all(&bytes).await {
                                    fail_item("Failed to write to file: {}",  &e);
                                    break;
                                }
                                downloaded_bytes += bytes.len() as u64;

                                // Update live stats!
                                if last_updated.elapsed().as_millis() > 500
                                    && let Some(mut item) = active_items.get_mut(&vault_id)
                                {
                                    let delta = (item.downloaded_bytes as u64).abs_diff(downloaded_bytes);
                                    item.total_bytes = total_bytes as i64;
                                    item.downloaded_bytes = downloaded_bytes as i64;
                                    if total_bytes > 0 {
                                        item.progress = (downloaded_bytes as f64 / total_bytes as f64) * 100.0;
                                    }
                                    // TODO: Calculate speed.
                                    item.speed_bps = (delta as f64 / 0.5).round() as i64 ;

                                    last_updated = Instant::now();
                                }
                            }
                            Ok(None) => {
                                tracing::info!("Download completed for vault_id: {}", vault_id);
                                if let Some(mut item) = active_items.get_mut(&vault_id) {
                                    item.downloaded_bytes = total_bytes as i64;
                                    item.progress = 100.0;
                                    item.status = VaultItemStatus::COMPLETED;
                                }
                                active_items.remove(&vault_id);
                                break;
                            }
                            Err(e) => {
                                fail_item("Error reading chunk: {}",  &e);
                                break;
                            }
                        }
                    }
                }
            }

            if let Err(e) = file.sync_all().await {
                tracing::warn!("Error syncing download file: {}", e);
            }
        });

        Ok(())
    }

    async fn pause(&self, vault_id: &Uuid) -> Result<()> {
        if let Some((_, token)) = self.cancel_tokens.remove(vault_id) {
            token.cancel();
        }
        Ok(())
    }

    async fn resume(&self, vault_id: &Uuid) -> Result<()> {
        // Just call `add` again! Our `add` logic automatically checks for file size
        // and sends the HTTP Range header, so it will resume where it left off.
        if let Some(item) = self.active_items.get(vault_id) {
            self.add(item.value()).await?;
        }
        Ok(())
    }

    async fn delete(&self, vault_id: &Uuid) -> Result<()> {
        // 1. Cancel ongoing download
        self.pause(vault_id).await?;

        // 2. Remove from stats
        if let Some((_, item)) = self.active_items.remove(vault_id) {
            // 3. Delete files from disk
            let _ = fs::remove_dir_all(&item.destination_path).await;
        }
        Ok(())
    }

    async fn get_stats(&self) -> Vec<VaultItem> {
        // Our background task updates active_items in real-time,
        // so we can just blindly return what's in the map!
        self.active_items
            .iter()
            .filter(|item| matches!(item.download_type, VaultDownloadType::DIRECT))
            .map(|item| item.value().clone())
            .collect()
    }
}
