use std::{collections::HashMap, path::PathBuf, sync::Arc};

use dashmap::DashMap;
use librqbit::{DhtSessionConfig, Session, SessionOptions, dht::DhtPersistenceConfig};
use loco_rs::Result;
use reqwest::Client;
use sea_orm::{DbConn, EntityTrait};
use tokio::{sync::Notify, task::JoinSet};
use uuid::Uuid;

use crate::{
    core::{ResultExt, constants::VAULT_LOC},
    downloaders::{DownloadEngine, direct::DirectDownloader, torrent::TorrentDownloader},
    models::vault::{self, VaultDownloadType, VaultItem, VaultItemStatus},
};

type SharedEngine = Arc<dyn DownloadEngine + Send + Sync>;
type SharedEngineMap = HashMap<VaultDownloadType, SharedEngine>;
type ActiveItemsMap = DashMap<Uuid, VaultItem>;

pub struct DownloadManager {
    pub active_items: Arc<ActiveItemsMap>,
    engines: Arc<SharedEngineMap>,
    pub wakeup: Arc<Notify>,
}

impl DownloadManager {
    async fn get_active_items(db: &DbConn) -> ActiveItemsMap {
        let map = DashMap::new();

        if let Ok(v) = vault::Entity::find().all(db).await {
            for item in v {
                if !matches!(
                    item.status,
                    VaultItemStatus::COMPLETED | VaultItemStatus::CANCELLED
                ) {
                    map.insert(item.id, item);
                }
            }
        }

        map
    }

    pub async fn new(db: &DbConn) -> Result<Arc<Self>> {
        let active_items = Arc::new(Self::get_active_items(db).await);

        tracing::info!("loading {} active items", active_items.len());

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
        let mut engines: SharedEngineMap = HashMap::new();

        let direct_engine = DirectDownloader::new(client.clone(), active_items.clone()).await;
        let torrent_engine = TorrentDownloader::new(session, active_items.clone()).await;

        engines.insert(VaultDownloadType::DIRECT, direct_engine);
        engines.insert(VaultDownloadType::TFILE, torrent_engine.clone());
        engines.insert(VaultDownloadType::MAGNET, torrent_engine);

        let engines = Arc::new(engines);

        let m = Arc::new(Self {
            active_items: active_items.clone(),
            engines: engines.clone(),
            wakeup: Arc::new(Notify::new()),
        });

        let bg_m = m.clone();

        tokio::spawn(async move {
            // 1. Extract items quickly to drop the DashMap lock
            let items: Vec<VaultItem> = active_items.iter().map(|v| v.value().clone()).collect();

            let mut set = JoinSet::new();

            // 2. Iterate and spawn isolated tasks
            for item in items {
                // Deref the DashMap guard and clone the engine so we own it for the async block
                if let Some(engine) = engines.get(&item.download_type).cloned() {
                    set.spawn(async move {
                        if let Err(e) = engine.add(&item).await {
                            tracing::error!("Failed to resume item {} for engine: {}", item.id, e);
                        }
                    });
                } else {
                    tracing::warn!(
                        "No download engine found for type {:?} for item {}",
                        item.download_type,
                        item.id
                    );
                }
            }

            // 3. Wait for all engine additions to finish
            while let Some(res) = set.join_next().await {
                if let Err(e) = res {
                    tracing::error!("Resume download task panicked during startup: {}", e);
                }
            }

            // 4. Wake the daemon after all items are loaded
            bg_m.wake_daemon();
        });

        Ok(m)
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
