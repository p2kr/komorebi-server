pub mod daemon;
pub mod direct;
pub mod manager;
pub mod torrent;

use std::fmt::Display;

use async_trait::async_trait;
use loco_rs::Result;
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
