pub mod daemon;
pub mod direct;
pub mod torrent;

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use dashmap::DashMap;
use librqbit::Session;
use loco_rs::Result;
use reqwest::Client;
use tokio::sync::Notify;
use uuid::Uuid;

use crate::{
    core::ResultExt,
    downloaders::{direct::DirectDownloader, torrent::TorrentDownloader},
    models::vault::{VaultDownloadType, VaultItem},
};

pub type SharedEngine = Arc<dyn DownloadEngine + Send + Sync>;

pub struct DownloadManager {
    pub active_items: Arc<DashMap<Uuid, VaultItem>>,
    engines: HashMap<VaultDownloadType, SharedEngine>,
    wakeup: Arc<Notify>,
}

#[async_trait]
pub trait DownloadEngine {
    async fn add(&self, vault_item: &VaultItem) -> Result<()>;

    async fn pause(&self, vault_id: &Uuid) -> Result<()>;

    async fn resume(&self, vault_id: &Uuid) -> Result<()>;

    async fn delete(&self, vault_id: &Uuid) -> Result<()>;

    async fn get_stats(&self) -> Vec<VaultItem>;
}

impl DownloadManager {
    pub async fn new() -> Result<Self> {
        let active_items = Arc::new(DashMap::<Uuid, VaultItem>::new());
        let client = Client::new();
        let session = Session::new("vault".into()).await.to_loco_err()?;

        let mut engines: HashMap<VaultDownloadType, SharedEngine> = HashMap::new();

        let direct_engine =
            Arc::new(DirectDownloader::new(client.clone(), active_items.clone()).await);
        let torrent_engine =
            Arc::new(TorrentDownloader::new(session.clone(), active_items.clone()).await);

        engines.insert(VaultDownloadType::DIRECT, direct_engine);
        engines.insert(VaultDownloadType::TFILE, torrent_engine.clone());
        engines.insert(VaultDownloadType::MAGNET, torrent_engine);

        Ok(Self {
            active_items,
            engines,
            wakeup: Arc::new(Notify::new()),
        })
    }

    pub fn wake_daemon(&self) {
        self.wakeup.notify_one();
    }

    #[must_use]
    pub fn get_engine(&self, download_type: &VaultDownloadType) -> Option<SharedEngine> {
        self.engines.get(download_type).cloned()
    }

    pub fn get_all_engines(&self) -> Vec<SharedEngine> {
        let mut unique = vec![];
        for engine in self.engines.values() {
            // We can compare pointers to deduplicate Arcs
            if !unique.iter().any(|e| Arc::ptr_eq(e, engine)) {
                unique.push(engine.clone());
            }
        }
        unique
    }
}
