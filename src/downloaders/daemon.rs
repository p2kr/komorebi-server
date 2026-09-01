use std::{sync::Arc, time::Duration};

use loco_rs::app::AppContext;
use sea_orm::{
    ActiveModelTrait,
    DbErr::{self},
};
use tokio::{sync::broadcast::Sender, time::interval};

use crate::{
    downloaders::{manager::DownloadManager, remove_vault_contents},
    models::{
        events::AppEvent,
        vault::{self, VaultItemStatus},
    },
};

pub fn start_daemon(ctx: AppContext, manager: Arc<DownloadManager>, ws: Sender<AppEvent>) {
    tokio::spawn(async move {
        tracing::info!("starting download manger polling daemon");
        let mut timer = interval(Duration::from_secs(2));
        loop {
            let mut all_stats = vec![];
            for engine in manager.get_all_engines() {
                let mut engine_stats = engine.get_stats().await;
                all_stats.append(&mut engine_stats);
            }

            // TODO:REMOVE: Not required since all items are polled every 2 secs
            let _ = ws.send(AppEvent::VaultActiveItems(all_stats.clone()));

            if all_stats.is_empty() {
                tracing::info!("no active downloads, waiting for wakeup signal");
                manager.notification().await;
                tracing::info!("wakeup signal received, resuming polling");
                continue;
            }

            for stat in all_stats.iter() {
                if stat.status == VaultItemStatus::COMPLETED {
                    tracing::info!("Download completed for: {}", stat.raw_title);
                    manager.active_items.remove(&stat.id);
                }
            }

            for item in all_stats {
                let id = item.id;
                let download_type = item.download_type.clone();
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
