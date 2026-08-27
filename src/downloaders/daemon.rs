use std::{sync::Arc, time::Duration};

use loco_rs::app::AppContext;
use sea_orm::ActiveModelTrait;
use tokio::{sync::broadcast::Sender, time::sleep};

use crate::{
    downloaders::DownloadManager,
    models::vault::{self, VaultItem},
};

pub fn start_daemon(ctx: AppContext, manager: Arc<DownloadManager>, ws: Sender<Vec<VaultItem>>) {
    tokio::spawn(async move {
        tracing::info!("starting download manger polling daemon");

        loop {
            let mut all_stats = vec![];
            for engine in manager.get_all_engines() {
                let mut engine_stats = engine.get_stats().await;
                all_stats.append(&mut engine_stats);
            }

            if all_stats.is_empty() {
                tracing::info!("no active downloads, waiting for wakeup signal");
                manager.wakeup.notified().await;
                tracing::info!("wakeup signal received, resuming polling");
                continue;
            }

            let _ = ws.send(all_stats.clone());

            for item in all_stats {
                let id = item.id;
                // save to db
                if let Err(e) = vault::ActiveModel::from(item).update(&ctx.db).await {
                    tracing::error!("failed to update vault item {}: {}", id, e);
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    });
}
