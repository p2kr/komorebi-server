use std::{collections::HashMap, sync::Arc};

use loco_rs::{Result, app::AppContext};
use sea_orm::{ActiveModelTrait, DbConn};
use tokio::{spawn, sync::broadcast, task::JoinHandle};
use uuid::Uuid;

use crate::{
    core::ResultExt,
    downloaders::manager::DownloadManager,
    dtos::{VaultStatus, events::AppEvent, vault::VaultSubItemDto},
    streaming::{
        StreamingEvent,
        processor::{MediaProcessor, cached_resolve_file_paths, remove_original},
    },
};

#[derive(Copy, Clone, Default)]
struct Progress {
    total: i64,
    success: i64,
    progress: f64,
    speed_bps: i64,
    eta_sec: i64,
}

pub fn start_monitoring(ctx: &AppContext) -> JoinHandle<()> {
    let db = ctx.db.clone();
    let processor: Arc<MediaProcessor> = ctx.shared_store.get().unwrap();
    let manager: Arc<DownloadManager> = ctx.shared_store.get().unwrap();
    let tx: broadcast::Sender<AppEvent> = ctx.shared_store.get().unwrap();

    tokio::spawn(async move {
        let manager = manager.clone();
        let mut rx = tx.subscribe();

        tracing::info!("starting monitoring");

        // [vault_id] : [total count, total progress, total speed, total eta]
        let mut freq_map: HashMap<Uuid, Progress> = HashMap::new();

        while !rx.is_closed() {
            if let Ok(AppEvent::StreamingEvents(event)) = rx.recv().await {
                match event {
                    StreamingEvent::Init { total, vault_id } => {
                        let entry = freq_map.entry(vault_id).or_insert(Progress {
                            total: total as i64,
                            ..Default::default()
                        });
                        entry.total = total as i64;
                    }
                    StreamingEvent::Complete {
                        sub_item_id,
                        vault_id,
                        is_last,
                    } => {
                        if is_last {
                            // Handle parent vault item aggregation
                            handle_vault_item(&manager, vault_id, &mut freq_map);

                            // Remove all related sub items
                            processor
                                .active_sub_items
                                .retain(|_, v| v.sub_item.vault_id != vault_id);
                            continue;
                        }

                        // Clone to release lock early.
                        let dto = if let Some(dto) = processor.active_sub_items.get(&sub_item_id) {
                            dto.clone()
                        } else {
                            continue;
                        };

                        match handle_sub_item(&db, &manager, &mut freq_map, dto).await {
                            Ok(_) => {
                                processor.active_sub_items.remove(&sub_item_id);
                            }
                            Err(e) => {
                                tracing::error!(error=%e, "error saving sub item/meta data to db");
                                if let Some((_, dto)) =
                                    processor.active_sub_items.remove(&sub_item_id)
                                {
                                    dto.sub_item
                                        .to_active_model_and_update_status(
                                            VaultStatus::FAILED,
                                            Some(e.to_string()),
                                        )
                                        .update(&db)
                                        .await
                                        .log_err_custom("error setting status to failed")
                                        .ok();
                                }
                            }
                        }
                    }
                    StreamingEvent::Progress {
                        sub_item_id,
                        total_size,
                        eta_secs,
                        speed,
                        progress,
                    } => {
                        let dto = if let Some(mut item) =
                            processor.active_sub_items.get_mut(&sub_item_id)
                        {
                            item.sub_item.status = VaultStatus::PROCESSING;
                            item.sub_item.total_bytes = total_size;
                            item.sub_item.speed_bps = speed;
                            item.sub_item.progress = progress;
                            item.sub_item.eta_seconds = eta_secs;
                            item.clone()
                        } else {
                            continue;
                        };

                        handle_sub_item(&db, &manager, &mut freq_map, dto)
                            .await
                            .ok();
                    }
                }
            }
        }
    })
}

/// Aggregates progress from sub item and updates parent vault item
async fn handle_sub_item(
    db: &DbConn,
    manager: &Arc<DownloadManager>,
    freq_map: &mut HashMap<Uuid, Progress>,
    dto: VaultSubItemDto,
) -> Result<()> {
    if let Some(metadata) = dto.metadata {
        metadata.save_vault_metadata_dto(db).await?;
    }

    dto.sub_item
        .to_active_model_and_update_progress()
        .update(db)
        .await?;

    let entry = freq_map.entry(dto.sub_item.vault_id).or_default();

    if entry.total == 0 {
        return Ok(());
    }

    entry.success += if dto.sub_item.status == VaultStatus::READY {
        1
    } else {
        0
    };
    let completed_progress = entry.success as f64 * 100.0;
    let in_flight_progress = if dto.sub_item.status == VaultStatus::READY {
        0.0
    } else {
        dto.sub_item.progress.max(0.0)
    };
    entry.progress =
        ((completed_progress + in_flight_progress) / entry.total as f64).clamp(0.0, 100.0);
    entry.speed_bps = entry.speed_bps.max(dto.sub_item.speed_bps); // Ideally should be min.
    entry.eta_sec = entry
        .eta_sec
        .max(dto.sub_item.eta_seconds.unwrap_or_default());

    // Aggregate all vault items in manager active items.
    for (vault_id, progress) in freq_map {
        if let Some(mut vault_item) = manager.active_items.get_mut(vault_id) {
            // No *100 because total_progress is already in percentage.
            vault_item.progress = progress.progress / progress.total as f64;
            vault_item.speed_bps = progress.speed_bps;
            vault_item.eta_seconds = Some(progress.eta_sec);
        }
    }
    manager.wake_daemon();
    Ok(())
}

fn handle_vault_item(
    manager: &Arc<DownloadManager>,
    vault_id: Uuid,
    freq_map: &mut HashMap<Uuid, Progress>,
) {
    // Handle manager
    if let Some(mut item) = manager.active_items.get_mut(&vault_id) {
        let freq = freq_map.remove(&vault_id).unwrap_or_default();

        item.speed_bps = 0;
        item.eta_seconds = Some(0);

        // Floating point approximation
        if freq.total == 0 {
            item.status = VaultStatus::FAILED;
            item.error_msg = Some("No playable media found".into());
        } else if freq.success == freq.total {
            item.progress = 100.0;
            item.status = VaultStatus::READY;
            item.error_msg = None;

            // Remove all downloaded files.
            let bg_m = manager.clone();
            let dest_path = item.dest_path.clone();
            let download_type = item.download_type.clone();
            spawn(async move {
                for (file_path, _) in cached_resolve_file_paths(&dest_path).await.iter() {
                    remove_original(file_path, &bg_m, &download_type, vault_id).await;
                }
            });
        } else if freq.success == 0 {
            item.progress = 0.0;
            item.status = VaultStatus::FAILED;
            item.error_msg = Some("All sub items failed".into());
        } else {
            item.progress = (freq.success as f64 / freq.total as f64) * 100.0;
            item.status = VaultStatus::PARTIAL;
            item.error_msg = Some(format!(
                "[{}/{}] Sub items failed",
                freq.total - freq.success,
                freq.total
            ))
        }
    }
    manager.wake_daemon();
}
