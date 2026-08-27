use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use loco_rs::{Error, Result};
use reqwest::{header, Client};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    core::{vault_path_resolver::get_file_path, ResultExt},
    downloaders::DownloadEngine,
    models::vault::VaultItem,
};

pub struct DirectDownloader {
    client: Client,
    cancel_tokens: DashMap<Uuid, CancellationToken>,
    active_items: Arc<DashMap<Uuid, VaultItem>>,
}

impl DirectDownloader {
    pub async fn new(client: Client, active_items: Arc<DashMap<Uuid, VaultItem>>) -> Self {
        Self {
            client,
            cancel_tokens: DashMap::new(),
            active_items,
        }
    }
}

#[async_trait]
impl DownloadEngine for DirectDownloader {
    async fn add(&self, vault_item: &VaultItem) -> Result<()> {
        let url = vault_item
            .source_url
            .clone()
            .ok_or_else(|| Error::BadRequest("Missing source URL".into()))?;

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
            // Determine if we need to resume
            let mut downloaded_bytes = 0;
            let mut req = client.get(&url);

            if let Ok(metadata) = fs::metadata(&file_path).await {
                downloaded_bytes = metadata.len();
                req = req.header(header::RANGE, format!("bytes={}-", downloaded_bytes));
            }

            let mut res = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to start direct download: {}", e);
                    return;
                }
            };

            let total_bytes = res.content_length().unwrap_or(0) + downloaded_bytes;

            let mut file = match OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!("Failed to open file: {}", e);
                    return;
                }
            };

            // Stream chunks
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        tracing::info!("Download paused/cancelled for vault_id: {}", vault_id);
                        break;
                    }
                    chunk = res.chunk() => {
                        match chunk {
                            Ok(Some(bytes)) => {
                                if let Err(e) = file.write_all(&bytes).await {
                                    tracing::error!("Failed to write to file: {}", e);
                                    break;
                                }
                                downloaded_bytes += bytes.len() as u64;

                                // Update live stats!
                                if let Some(mut item) = active_items.get_mut(&vault_id) {
                                    item.total_bytes = total_bytes as i64;
                                    item.downloaded_bytes = downloaded_bytes as i64;
                                    if total_bytes > 0 {
                                        item.progress = (downloaded_bytes as f64 / total_bytes as f64) * 100.0;
                                    }
                                }
                            }
                            Ok(None) => {
                                tracing::info!("Download completed for vault_id: {}", vault_id);
                                active_items.remove(&vault_id);
                                break;
                            }
                            Err(e) => {
                                tracing::error!("Error reading chunk: {}", e);
                                break;
                            }
                        }
                    }
                }
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
            .map(|item| item.value().clone())
            .collect()
    }
}
