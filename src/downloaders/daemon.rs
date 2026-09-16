use std::{sync::Arc, time::Duration};

use futures::{StreamExt, stream};
use loco_rs::app::AppContext;
use sea_orm::{
    ActiveModelTrait, DbConn,
    DbErr::{self},
    EntityTrait,
};
use tokio::{
    task::JoinHandle,
    time::{MissedTickBehavior, interval},
};

const MAX_CONCURRENT_DB_OPS: usize = 10;

use crate::{
    downloaders::{manager::DownloadManager, remove_vault_contents},
    dtos::VaultStatus,
    models::vault::{self, VaultItem},
    streaming::processor::MediaProcessor,
};

pub fn start_daemon(ctx: &AppContext) -> JoinHandle<()> {
    let db = ctx.db.clone();
    let processor: Arc<MediaProcessor> = ctx.shared_store.get().unwrap();
    let manager: Arc<DownloadManager> = ctx.shared_store.get().unwrap();
    tokio::spawn(async move {
        // let _ws: Sender<AppEvent> = ctx.shared_store.get().unwrap();

        tracing::info!("starting download manger polling daemon");

        let mut timer = interval(Duration::from_secs(2));
        timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            timer.tick().await;

            if manager.active_items.is_empty() {
                tracing::info!("no active downloads, waiting for wakeup signal");
                manager.notification().await;
                tracing::info!("wakeup signal received, resuming polling");
                timer.reset();
                continue;
            }

            for engine in manager.get_all_engines() {
                engine.update_stats();
            }

            let active_items: Vec<VaultItem> = manager
                .active_items
                .iter_mut()
                .map(|mut entry| {
                    take_action_on_status(entry.value_mut(), db.clone(), &processor, &manager);
                    entry.value().clone()
                })
                .collect();

            stream::iter(active_items.into_iter().map(|item| {
                let db = db.clone();
                let manager = manager.clone();
                async move { save_to_db(&item, &db, &manager).await }
            }))
            .buffer_unordered(MAX_CONCURRENT_DB_OPS)
            .collect::<Vec<_>>()
            .await;
        }
    })
}

fn take_action_on_status(
    item: &mut VaultItem,
    db: DbConn,
    processor: &Arc<MediaProcessor>,
    manager: &Arc<DownloadManager>,
) {
    if item.status == VaultStatus::COMPLETED {
        tracing::info!("Download completed for: {}", item.title);

        // Send it to post-process
        item.status = VaultStatus::PROCESSING;
        item.speed_bps = 0;
        item.progress = 100.0;
        item.eta_seconds = None;

        let bg_proc = processor.clone();
        let bg_manager = manager.clone();
        let bg_item = item.clone();

        tokio::spawn(async move {
            bg_proc.start(&db, &bg_item, bg_manager).await;
        });
    } else if matches!(
        item.status,
        VaultStatus::READY | VaultStatus::FAILED | VaultStatus::CANCELLED | VaultStatus::PARTIAL
    ) {
        tracing::info!("Processing {:?} for: {}", item.status, item.title);
        if item.status == VaultStatus::READY {
            item.progress = 100.0;
        }
        item.speed_bps = 0;
        item.eta_seconds = None;

        // Delete this item from db
    }
}

async fn save_to_db(item: &VaultItem, db: &DbConn, manager: &Arc<DownloadManager>) {
    if item.status == VaultStatus::CANCELLED {
        // Delete
        match vault::Entity::delete_by_id(item.id).exec(db).await {
            Ok(_) => {
                manager.active_items.remove(&item.id);
                // asynchronous inside
                remove_vault_contents(item);
            }
            Err(e) => {
                tracing::error!(error=%e, "failed to delete {}", item.id);
            }
        };

        return;
    }

    // save to db
    if let Err(e) = item.to_active_model_and_update_progress().update(db).await {
        match e {
            DbErr::RecordNotUpdated => {
                tracing::warn!(
                    "Vault item {} was deleted from DB; removing from active downloads",
                    item.id
                );
                if let Some(engine) = manager.get_engine(&item.download_type) {
                    let _ = engine.delete(item).await;
                }
                manager.active_items.remove(&item.id);

                // also delete its files
                remove_vault_contents(item);
            }
            _ => tracing::error!(error=%e, "failed to update vault item {}", item.id),
        }
    } else {
        // SUCCESS: If the item was a terminal status, evict it from memory now.
        if matches!(
            item.status,
            VaultStatus::READY | VaultStatus::FAILED | VaultStatus::PARTIAL
        ) {
            manager.active_items.remove(&item.id);
        }
    }
}
