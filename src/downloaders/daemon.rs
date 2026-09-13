use std::{sync::Arc, time::Duration};

use sea_orm::{
    ActiveModelTrait, DbConn,
    DbErr::{self},
    EntityTrait,
};
use tokio::{task::JoinHandle, time::interval};

use crate::{
    downloaders::{manager::DownloadManager, remove_vault_contents},
    dtos::VaultStatus,
    models::vault::{self, VaultItem},
    streaming::processor::MediaProcessor,
};

pub fn start_daemon(
    db: DbConn,
    processor: Arc<MediaProcessor>,
    manager: Arc<DownloadManager>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // let _ws: Sender<AppEvent> = ctx.shared_store.get().unwrap();

        tracing::info!("starting download manger polling daemon");
        let mut timer = interval(Duration::from_secs(2));
        loop {
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
                .iter()
                .map(|v| v.value().clone())
                .collect();

            for item in active_items.iter() {
                if item.status == VaultStatus::COMPLETED {
                    tracing::info!("Download completed for: {}", item.title);
                    // Send it to post-process
                    if let Some(mut it) = manager.active_items.get_mut(&item.id) {
                        it.status = VaultStatus::PROCESSING;
                        let processor = processor.clone();
                        let manager = manager.clone();
                        let it_clone = it.clone();
                        tokio::spawn(async move {
                            processor.start(&it_clone, manager).await;
                        });
                    }
                } else if matches!(
                    item.status,
                    VaultStatus::READY | VaultStatus::FAILED | VaultStatus::CANCELLED
                ) {
                    tracing::info!("Processing {:?} for: {}", item.status, item.title);
                    manager.active_items.remove(&item.id);
                }
            }

            // Update progress in db
            for item in active_items {
                let id = item.id;
                let download_type = item.download_type.clone();

                if item.status == VaultStatus::CANCELLED {
                    // Delete
                    match vault::Entity::delete_by_id(item.id).exec(&db).await {
                        Ok(_) => {
                            manager.active_items.remove(&item.id);
                            remove_vault_contents(item);
                        }
                        Err(e) => {
                            tracing::error!(error=%e, "failed to delete {}", item.id);
                        }
                    };

                    continue;
                }

                // save to db
                if let Err(e) = vault::ActiveModel::from(item.clone())
                    .update_progress_mut()
                    .update(&db)
                    .await
                {
                    match e {
                        DbErr::RecordNotUpdated => {
                            tracing::warn!(
                                "Vault item {} was deleted from DB; removing from active downloads",
                                id
                            );
                            if let Some(engine) = manager.get_engine(&download_type) {
                                let _ = engine.delete(&id).await;
                            }
                            manager.active_items.remove(&id);

                            // also delete its files
                            remove_vault_contents(item);
                        }
                        _ => tracing::error!("failed to update vault item {}: {}", id, e),
                    }
                }
            }

            timer.tick().await;
        }
    })
}
