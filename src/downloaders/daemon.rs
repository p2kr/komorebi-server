use std::{sync::Arc, time::Duration};

use loco_rs::app::AppContext;
use sea_orm::{
    ActiveModelTrait,
    DbErr::{self},
    EntityTrait,
};
use tokio::{sync::broadcast::Sender, time::interval};

use crate::{
    downloaders::{manager::DownloadManager, remove_vault_contents},
    models::{
        events::AppEvent,
        vault::{self, VaultItem, VaultItemStatus},
    },
    streaming::processor::Streaming,
};

pub fn start_daemon(ctx: AppContext, manager: Arc<DownloadManager>, _ws: Sender<AppEvent>) {
    tokio::spawn(async move {
        tracing::info!("starting download manger polling daemon");
        let mut timer = interval(Duration::from_secs(2));
        loop {
            for engine in manager.get_all_engines() {
                engine.update_stats();
            }

            let active_items: Vec<VaultItem> = manager
                .active_items
                .iter()
                .map(|v| v.value().clone())
                .collect();

            if active_items.is_empty() {
                tracing::info!("no active downloads, waiting for wakeup signal");
                manager.notification().await;
                tracing::info!("wakeup signal received, resuming polling");
                continue;
            }

            for item in active_items.iter() {
                if item.status == VaultItemStatus::COMPLETED {
                    tracing::info!("Download completed for: {}", item.raw_title);
                    // Send it to post-process
                    if let Some(mut it) = manager.active_items.get_mut(&item.id) {
                        it.status = VaultItemStatus::PROCESSING;
                        Streaming::post_process(manager.clone(), it.clone());
                    }
                } else if matches!(
                    item.status,
                    VaultItemStatus::READY | VaultItemStatus::FAILED | VaultItemStatus::CANCELLED
                ) {
                    tracing::info!("Processing {:?} for: {}", item.status, item.raw_title);
                    manager.active_items.remove(&item.id);
                }
            }

            // Update progress in db
            for item in active_items {
                let id = item.id;
                let download_type = item.download_type.clone();

                if item.status == VaultItemStatus::CANCELLED {
                    // Delete
                    match vault::Entity::delete_by_id(item.id).exec(&ctx.db).await {
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
                    .update(&ctx.db)
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
    });
}
