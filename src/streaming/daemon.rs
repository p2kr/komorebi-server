use std::{collections::HashMap, sync::Arc, time::Duration};

use itertools::Itertools;
use sea_orm::{ActiveModelTrait, IntoActiveModel};
use tokio::{
    task::JoinHandle,
    time::{Instant, interval},
};
use uuid::Uuid;

use crate::{downloaders::manager::DownloadManager, streaming::processor::MediaProcessor};

pub fn start_monitoring(
    processor: Arc<MediaProcessor>,
    manager: Arc<DownloadManager>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(2));
        let mut last_db_sync = Instant::now();
        loop {
            if processor.active_sub_items.is_empty() {
                processor.notification().await;
                timer.reset();
                continue;
            }

            let sub_items = processor
                .active_sub_items
                .iter()
                .map(|v| v.value().clone())
                .collect_vec();

            // [vault_id] : [total count, total progress, total speed, total eta]
            let mut vault_aggregates: HashMap<Uuid, (i64, f64, i64, i64)> = HashMap::new();

            let should_sync_db = last_db_sync.elapsed() >= Duration::from_secs(10);
            for sub_item in sub_items {
                let vault_id = sub_item.vault_id;
                let speed_bps = sub_item.speed_bps;
                let progress = sub_item.progress;
                let eta_sec = sub_item.eta_seconds.unwrap_or_default();

                // Update in db
                if should_sync_db {
                    sub_item
                        .into_active_model()
                        .update_progress_mut()
                        .update(&processor.db)
                        .await
                        .ok();
                }

                let entry = vault_aggregates.entry(vault_id).or_insert((0, 0.0, 0, 0));

                entry.0 += 1;
                entry.1 += progress;
                entry.2 += speed_bps;
                entry.3 = entry.3.max(eta_sec);
            }

            if should_sync_db {
                last_db_sync = Instant::now();
            }

            for (vault_id, (total_count, total_progress, total_speed, total_eta)) in
                vault_aggregates
            {
                if let Some(mut vault_item) = manager.active_items.get_mut(&vault_id) {
                    vault_item.progress = total_progress / total_count as f64;
                    vault_item.speed_bps = total_speed;
                    vault_item.eta_seconds = Some(total_eta);
                }
            }

            manager.wake_daemon();
            timer.tick().await;
        }
    })
}
