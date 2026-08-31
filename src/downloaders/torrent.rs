use std::{fmt::Display, sync::Arc};

use dashmap::DashMap;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session};
use loco_rs::prelude::async_trait;
use loco_rs::{Error, Result};
use reqwest::{Client, Url};
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::core::client::get_common_trackers;
use crate::{
    core::ResultExt,
    downloaders::DownloadEngine,
    models::vault::{VaultDownloadType, VaultItem, VaultItemStatus},
};

pub struct TorrentDownloader {
    client: Client,
    session: Arc<Session>,
    active_items: Arc<DashMap<Uuid, VaultItem>>,
    handles: DashMap<Uuid, Arc<ManagedTorrent>>,
}

const TRACKERS: OnceCell<Vec<String>> = OnceCell::const_new();

impl Display for TorrentDownloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TorrentDownloader[session={}, active_items={}, handles={}]",
            self.session.client_name_and_version(),
            self.active_items.len(),
            self.handles.len()
        )
    }
}

impl TorrentDownloader {
    pub async fn new(
        client: Client,
        session: Arc<Session>,
        active_items: Arc<DashMap<Uuid, VaultItem>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            client,
            session,
            active_items,
            handles: DashMap::new(),
        })
    }
}

#[async_trait]
impl DownloadEngine for TorrentDownloader {
    async fn add(&self, vault_item: &VaultItem) -> Result<()> {
        let url = Url::parse(&vault_item.source_url).to_loco_string()?;

        // TODO: Truncate to max thousand/hundred to avoid cloning performance issues?
        let trackers = TRACKERS
            .get_or_init(|| get_common_trackers(&self.client))
            .await
            .clone();

        // 1. Configure the torrent
        let opts = AddTorrentOptions {
            paused: false,
            output_folder: Some(vault_item.destination_path.clone()),
            overwrite: true,
            trackers: Some(trackers),
            ..Default::default()
        };

        // 2. Add it to the librqbit session
        let add_request = AddTorrent::from_url(url.to_string());
        let response = self
            .session
            .add_torrent(add_request, Some(opts))
            .await
            .to_loco_string()?;

        // 3. Save the handle so we can pause/resume/stat it later
        match response {
            AddTorrentResponse::Added(_, handle) => {
                self.handles.insert(vault_item.id, handle);
                self.active_items.insert(vault_item.id, vault_item.clone());
            }
            AddTorrentResponse::AlreadyManaged(_, _) => {
                return Err(Error::Unauthorized("Torrent already added".into()));
            }
            _ => return Err(Error::BadRequest("Failed to add torrent to session".into())),
        }

        Ok(())
    }

    async fn pause(&self, vault_id: &Uuid) -> Result<()> {
        if let Some(handle) = self.handles.get(vault_id) {
            self.session.pause(handle.value()).await.to_loco_string()?;
            if let Some(mut v) = self.active_items.get_mut(vault_id) {
                v.status = VaultItemStatus::PAUSED;
            }
        }
        Ok(())
    }

    async fn resume(&self, vault_id: &Uuid) -> Result<()> {
        if let Some(handle) = self.handles.get(vault_id) {
            self.session
                .unpause(handle.value())
                .await
                .to_loco_string()?;
            if let Some(mut v) = self.active_items.get_mut(vault_id) {
                v.status = VaultItemStatus::PENDING;
            }
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
                .to_loco_string()?;
        }
        Ok(())
    }

    async fn get_stats(&self) -> Vec<VaultItem> {
        let mut stats = vec![];

        for entry in self.handles.iter() {
            let (vault_id, handle) = (entry.key(), entry.value());
            let t_stats = handle.stats();

            // Fetch the VaultItem and update it with the live librqbit stats!
            if let Some(mut item) = self.active_items.get_mut(vault_id)
                && !handle.is_paused()
                && matches!(
                    item.status,
                    VaultItemStatus::DOWNLOADING | VaultItemStatus::PENDING
                )
                && matches!(
                    item.download_type,
                    VaultDownloadType::MAGNET | VaultDownloadType::TFILE
                )
            {
                item.total_bytes = t_stats.total_bytes as i64;

                item.downloaded_bytes = t_stats.progress_bytes as i64;

                if item.total_bytes > 0 {
                    item.progress =
                        (item.downloaded_bytes as f64 / item.total_bytes as f64) * 100.0;
                }

                let live_speed = t_stats.live.unwrap_or_default().download_speed.as_bytes() as i64;
                item.speed_bps = live_speed;

                item.eta_seconds = item
                    .total_bytes
                    .saturating_sub(item.downloaded_bytes)
                    .checked_div(item.speed_bps);

                // tracing::debug!(
                //     "downloaded={}, total={}, speed={}",
                //     item.downloaded_bytes,
                //     item.total_bytes,
                //     item.speed_bps
                // );

                if t_stats.finished || item.progress == 100f64 {
                    item.status = VaultItemStatus::COMPLETED;
                } else {
                    item.status = VaultItemStatus::DOWNLOADING;
                }

                stats.push(item.to_owned());
            }
        }
        stats
    }

    async fn stop(&self) {
        self.session.stop().await;
    }
}
