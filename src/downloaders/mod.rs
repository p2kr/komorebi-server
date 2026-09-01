pub mod daemon;
pub mod direct;
pub mod manager;
pub mod torrent;

use std::fmt::Display;

use loco_rs::{Result, prelude::async_trait};
use tokio::fs;
use uuid::Uuid;

use crate::models::vault::VaultItem;

#[async_trait]
pub trait DownloadEngine: Display {
    async fn add(&self, vault_item: &VaultItem) -> Result<()>;

    async fn pause(&self, vault_id: &Uuid) -> Result<()>;

    async fn resume(&self, vault_id: &Uuid) -> Result<()>;

    async fn delete(&self, vault_id: &Uuid) -> Result<()>;

    async fn get_stats(&self) -> Vec<VaultItem>;

    /// stop session/client
    async fn stop(&self) {}
}

pub fn remove_vault_contents(item: VaultItem) {
    tokio::spawn(async move {
        if let Err(e) = fs::remove_dir_all(&item.destination_path).await {
            tracing::error!(
                "Failed to delete download path {} for vault item {}: {}",
                item.destination_path,
                item.id,
                e
            );
        }
    });
}
