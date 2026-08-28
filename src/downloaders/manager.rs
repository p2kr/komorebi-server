use std::{collections::HashMap, path::PathBuf, sync::Arc};

use dashmap::DashMap;
use librqbit::{DhtSessionConfig, Session, SessionOptions, dht::DhtPersistenceConfig};
use loco_rs::Result;
use reqwest::Client;
use sea_orm::{DbConn, EntityTrait};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::{
    core::{ResultExt, constants::VAULT_LOC},
    downloaders::{DownloadEngine, direct::DirectDownloader, torrent::TorrentDownloader},
    models::vault::{self, VaultDownloadType, VaultItem},
};

pub type SharedEngine = Arc<dyn DownloadEngine + Send + Sync>;

pub struct DownloadManager {
    pub active_items: Arc<DashMap<Uuid, VaultItem>>,
    engines: HashMap<VaultDownloadType, SharedEngine>,
    pub wakeup: Arc<Notify>,
}

impl DownloadManager {
    async fn get_active_items(db: &DbConn) -> DashMap<Uuid, VaultItem> {
        let map = DashMap::new();

        if let Ok(v) = vault::Entity::find().all(db).await {
            for item in v {
                map.insert(item.id, item);
            }
        }

        map
    }

    pub async fn new(db: &DbConn) -> Result<Self> {
        let active_items = Arc::new(Self::get_active_items(db).await);
        let client = Client::new();

        let session_opt_default = SessionOptions {
            dht: Some(DhtSessionConfig {
                persistence: Some(DhtPersistenceConfig {
                    config_filename: Some(PathBuf::from("assets/dht.json")),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let session_opt_backup = SessionOptions {
            dht: Some(DhtSessionConfig {
                persistence: None,
                ..Default::default()
            }),
            ..Default::default()
        };

        let vault_loc = PathBuf::from(VAULT_LOC.clone());

        let mut session_res = Session::new_with_opts(vault_loc.clone(), session_opt_default).await;

        if let Err(e) = session_res {
            tracing::warn!("failed to get persistent librqbit session: {}", e);
            session_res = Session::new_with_opts(vault_loc, session_opt_backup).await;
        }

        let session = session_res.to_loco_err()?;

        let mut engines: HashMap<VaultDownloadType, SharedEngine> = HashMap::new();

        let direct_engine =
            Arc::new(DirectDownloader::new(client.clone(), active_items.clone()).await);
        let torrent_engine = Arc::new(TorrentDownloader::new(session, active_items.clone()).await);

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
