use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session,
};
// You might need to import the handle type depending on your librqbit version:
use loco_rs::{Error, Result};
use uuid::Uuid;

use crate::{core::ResultExt, downloaders::DownloadEngine, models::vault::VaultItem};

pub struct TorrentDownloader {
    session: Arc<Session>,
    active_items: Arc<DashMap<Uuid, VaultItem>>,
    handles: DashMap<Uuid, Arc<ManagedTorrent>>,
}

impl TorrentDownloader {
    pub async fn new(session: Arc<Session>, active_items: Arc<DashMap<Uuid, VaultItem>>) -> Self {
        Self {
            session,
            active_items,
            handles: DashMap::new(),
        }
    }
}

#[async_trait]
impl DownloadEngine for TorrentDownloader {
    async fn add(&self, vault_item: &VaultItem) -> Result<()> {
        let url = vault_item
            .source_url
            .as_ref()
            .ok_or_else(|| Error::BadRequest("Missing source URL".into()))?;

        // 1. Configure the torrent
        let opts = AddTorrentOptions {
            paused: false,
            output_folder: Some(vault_item.destination_path.clone()),
            ..Default::default()
        };

        // 2. Add it to the librqbit session
        let add_request = AddTorrent::from_url(url);
        let response = self
            .session
            .add_torrent(add_request, Some(opts))
            .await
            .to_loco_err()?;

        // 3. Save the handle so we can pause/resume/stat it later
        match response {
            AddTorrentResponse::Added(_, handle)
            | AddTorrentResponse::AlreadyManaged(_, handle) => {
                self.handles.insert(vault_item.id, handle);
                self.active_items.insert(vault_item.id, vault_item.clone());
            }
            _ => return Err(Error::BadRequest("Failed to add torrent to session".into())),
        }

        Ok(())
    }

    async fn pause(&self, vault_id: &Uuid) -> Result<()> {
        if let Some(handle) = self.handles.get(vault_id) {
            self.session.pause(handle.value()).await.to_loco_err()?;
        }
        Ok(())
    }

    async fn resume(&self, vault_id: &Uuid) -> Result<()> {
        if let Some(handle) = self.handles.get(vault_id) {
            self.session.unpause(handle.value()).await.to_loco_err()?;
        }
        Ok(())
    }

    async fn delete(&self, vault_id: &Uuid) -> Result<()> {
        // 1. Remove it from our tracking maps
        if let Some((_, handle)) = self.handles.remove(vault_id) {
            self.active_items.remove(vault_id);

            // 2. Tell librqbit to delete it entirely (including files!)
            self.session
                .delete(handle.info_hash().into(), true)
                .await
                .to_loco_err()?;
        }
        Ok(())
    }

    async fn get_stats(&self) -> Vec<VaultItem> {
        let mut stats = vec![];

        for entry in self.handles.iter() {
            let (vault_id, handle) = (entry.key(), entry.value());
            let t_stats = handle.stats();

            // Fetch the VaultItem and update it with the live librqbit stats!
            if let Some(mut item) = self.active_items.get_mut(vault_id) {
                item.total_bytes = t_stats.total_bytes as i64;

                let downloaded = t_stats.total_bytes.saturating_sub(t_stats.progress_bytes);
                item.downloaded_bytes = downloaded as i64;

                if item.total_bytes > 0 {
                    item.progress = (downloaded as f64 / item.total_bytes as f64) * 100.0;
                }

                item.speed_bps = t_stats.live.unwrap_or_default().download_speed.as_bytes();

                stats.push(item.clone());
            }
        }
        stats
    }
}
