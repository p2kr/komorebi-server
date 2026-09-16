pub mod daemon;
pub mod direct;
pub mod manager;
pub mod torrent;

use std::{any::Any, fmt::Display};

use loco_rs::{Error, Result, prelude::async_trait};
use tokio::{fs, task::JoinHandle};

use crate::models::vault::VaultItem;

#[async_trait]
pub trait DownloadEngine: Display {
    fn as_any(&self) -> &dyn Any;

    async fn add(&self, vault_item: &VaultItem) -> Result<()>;

    async fn pause(&self, vault_id: &VaultItem) -> Result<()>;

    async fn resume(&self, vault_id: &VaultItem) -> Result<()>;

    async fn delete(&self, vault_id: &VaultItem) -> Result<()>;

    fn update_stats(&self);

    /// stop session/client
    async fn stop(&self) {}
}

pub enum DownloadEngineEvents {
    AlreadyManaged,
    FailedToResume,
    DownloadComplete,
    FailedToAdd,
    Error(Error),
}

impl From<Error> for DownloadEngineEvents {
    fn from(value: Error) -> Self {
        Self::Error(value)
    }
}

pub fn remove_vault_contents(item: &VaultItem) -> JoinHandle<()> {
    let dest_path = item.dest_path.clone();
    let id = item.id;
    tokio::spawn(async move {
        if let Err(e) = fs::remove_dir_all(&dest_path).await {
            tracing::error!(
                "Failed to delete download path {} for vault item {}: {}",
                dest_path,
                id,
                e
            );
        }
    })
}
