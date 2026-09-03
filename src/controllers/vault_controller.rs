use async_stream::stream;
use axum::body::Body;
use axum::http::Request;
use axum::response::{Sse, sse::KeepAlive};
use loco_rs::prelude::*;
use reqwest::Url;
use serde::Deserialize;
use std::path::PathBuf;
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast::error::RecvError;
use tokio::{sync::broadcast::Sender, time::interval};
use tower_http::services::ServeFile;
use ts_rs::TS;
use uuid::Uuid;

use crate::downloaders::remove_vault_contents;
use crate::streaming::processor::Streaming;
use crate::{
    controllers::success,
    core::vault_path_resolver::get_dest_path,
    downloaders::manager::DownloadManager,
    loco_err, loco_err_msg,
    models::{
        crawler::{CrawlerResult, ParsedTitle},
        events::AppEvent,
        media::MediaType,
        vault::{self, VaultDownloadType, VaultItem, VaultItemStatus},
    },
};

// --- JSON Payloads ---
#[derive(Deserialize, TS)]
#[ts(export)]
pub struct VaultActionPayload {
    vault_id: Uuid,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct VaultAddPayload {
    crawler_result: CrawlerResult,
    user_id: Uuid,
}

// --- Endpoints ---
fn get_download_type_from_url(url: &str) -> Result<VaultDownloadType> {
    let u = Url::parse(url).map_err(|e| Error::BadRequest(format!("Invalid Url: {}", e)))?;

    if u.scheme() == "magnet" {
        Ok(VaultDownloadType::MAGNET)
    } else if matches!(u.scheme(), "http" | "https") && u.path().ends_with(".torrent") {
        Ok(VaultDownloadType::TFILE)
    } else if matches!(u.scheme(), "http" | "https") {
        Ok(VaultDownloadType::DIRECT)
    } else {
        Err(Error::BadRequest(format!(
            "Unsupported URL scheme: {}",
            u.scheme()
        )))
    }
}

fn get_media_id(_parsed_title: &ParsedTitle) -> Result<String> {
    // TODO: Connect with anilist/mal and find the best match
    // todo!()
    Ok("123".into())
}

fn get_media_type(parsed_title: &ParsedTitle) -> Result<MediaType> {
    // TODO: Look at file extensions to decide. In torrent it may be in subdirectory.
    // todo!()
    if ["mp4", "mkv", "av1"].iter().any(|f| {
        parsed_title
            .file_extension
            .contains(f.to_lowercase().as_str())
    }) {
        return Ok(MediaType::Anime);
    }
    // Err(Error::Message("Invalid media type".into()))
    Ok(MediaType::Anime)
}

#[debug_handler]
pub async fn add(
    State(ctx): State<AppContext>,
    Json(params): Json<VaultAddPayload>,
) -> Result<Response> {
    // Check if already in db
    let existing_items = vault::Entity::find()
        .filter(vault::Column::SourceUrl.eq(params.crawler_result.link.clone()))
        .all(&ctx.db)
        .await?;

    if !existing_items.is_empty() {
        return loco_err!("Item already exists in vault/queue");
    }

    let title = params
        .crawler_result
        .parsed_title
        .title
        .first()
        .ok_or(Error::BadRequest("No title found".into()))?;

    let download_type = get_download_type_from_url(&params.crawler_result.link)?;

    let vault_id = Uuid::now_v7();

    let vault_item = VaultItem {
        id: vault_id,
        user_id: params.user_id,
        destination_path: get_dest_path(&vault_id),
        media_type: Some(get_media_type(&params.crawler_result.parsed_title)?),
        media_id: get_media_id(&params.crawler_result.parsed_title)?,
        title: title.clone(),
        source_url: params.crawler_result.link.clone(),
        download_type: download_type.clone(),
        raw_title: params.crawler_result.title,
        season: params.crawler_result.parsed_title.season.first().cloned(),
        episode: params.crawler_result.parsed_title.episode.first().cloned(),
        ..Default::default()
    };

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    // Delegate to the correct engine
    let engine = manager
        .get_engine(&download_type)
        .ok_or(loco_err_msg!("Download engine not found for this item"))?;

    // Create a new VaultItem in the database with PENDING status
    let inserted_item = vault::ActiveModel::from(vault_item).insert(&ctx.db).await?;

    let bg_inserted_item = inserted_item.clone();
    tokio::spawn(async move {
        if let Err(e) = engine.add(&bg_inserted_item).await {
            tracing::error!("Failed to add new torrent {}: {}", bg_inserted_item.id, e);

            // Fail in db
            vault::ActiveModel::from(bg_inserted_item)
                .update_status(VaultItemStatus::FAILED, Some(e.to_string()))
                .update(&ctx.db)
                .await
                .inspect_err(|e| tracing::error!("Error adding torrent {}", e))
                .ok();
        } else {
            // Wake up the daemon so it starts polling this new download!
            manager.wake_daemon();
        }
    });

    success(inserted_item)
}

#[debug_handler]
pub async fn pause(
    State(ctx): State<AppContext>,
    Json(params): Json<VaultActionPayload>,
) -> Result<Response> {
    // Fetch the item to ensure it exists and we know its download_type
    let item = vault::Entity::find_by_id(params.vault_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    if matches!(
        item.status,
        VaultItemStatus::COMPLETED
            | VaultItemStatus::PROCESSING
            | VaultItemStatus::READY
            | VaultItemStatus::PENDING
            | VaultItemStatus::PAUSED
    ) {
        return loco_err!("Cannot pause a completed/pending/paused download");
    }

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();
    let engine = manager
        .get_engine(&item.download_type)
        .ok_or(Error::NotFound)?;

    tokio::spawn(async move {
        if let Err(e) = engine.pause(&params.vault_id).await {
            tracing::error!("Failed to pause download {}: {}", params.vault_id, e);
        }
        manager.wake_daemon();
    });

    success(serde_json::json!("Paused download"))
}

#[debug_handler]
pub async fn resume(
    State(ctx): State<AppContext>,
    Json(params): Json<VaultActionPayload>,
) -> Result<Response> {
    // Fetch the item to ensure it exists and we know its download_type
    let item = vault::Entity::find_by_id(params.vault_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    if matches!(
        item.status,
        |VaultItemStatus::READY| VaultItemStatus::PENDING | VaultItemStatus::DOWNLOADING
    ) {
        return loco_err!("Cannot resume a completed/ongoing download");
    }

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    let engine = manager
        .get_engine(&item.download_type)
        .ok_or(Error::NotFound)?;

    tokio::spawn(async move {
        if let Err(e) = engine.resume(&params.vault_id).await {
            tracing::error!("Failed to resume download {}: {}", params.vault_id, e);
        }
        manager.wake_daemon();
    });

    success(serde_json::json!("Download Queued for Resume"))
}

#[debug_handler]
pub async fn delete(
    State(ctx): State<AppContext>,
    Json(params): Json<VaultActionPayload>,
) -> Result<Response> {
    // Fetch the item to ensure it exists and we know its download_type
    let item = vault::Entity::find_by_id(params.vault_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    let engine = manager
        .get_engine(&item.download_type)
        .ok_or(Error::NotFound)?;

    tokio::spawn(async move {
        if let Err(e) = engine.delete(&item.id).await {
            tracing::error!(
                "Failed to delete/cancel download {}: {}",
                params.vault_id,
                e
            );
        }
        // Remove from db:
        // We can remove it from the db regardless of whether the engine delete succeeded or not,
        // because if the engine delete failed, it might be because the download was already completed
        // or not found, in which case we still want to remove it from our vault.
        if let Err(e) = vault::Entity::delete_by_id(item.id).exec(&ctx.db).await {
            tracing::error!(
                "Failed to delete vault item {} from db: {}",
                params.vault_id,
                e
            );
        }

        manager.wake_daemon();

        // also delete the destination path if it exists
        remove_vault_contents(item);
    });

    success(serde_json::json!("Deleted/Cancelled download"))
}

#[axum::debug_handler]
pub async fn active(State(ctx): State<AppContext>) -> impl IntoResponse {
    // Fetch the broadcast receiver from shared store
    let tx = ctx.shared_store.get::<Sender<AppEvent>>().unwrap();

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();
    let initial_items: Vec<VaultItem> = manager
        .active_items
        .iter()
        .map(|v| v.value().clone())
        .collect();

    let stream = stream! {
        yield AppEvent::VaultActiveItems(initial_items).to_sse();

        let mut rx = tx.subscribe();
        loop {
            match rx.recv().await {
              Ok(v) =>  yield v.to_sse(),
              Err(RecvError::Closed) => break,
              _ => continue
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive-active"),
    )
}

#[axum::debug_handler]
pub async fn all(State(ctx): State<AppContext>) -> impl IntoResponse {
    let db = ctx.db.clone();

    let stream = stream! {
        let mut timer = interval(Duration::from_secs(2));
        loop {
            timer.tick().await;

            yield match vault::Entity::find()
                .all(&db)
                .await
            {
                Ok(v) => AppEvent::VaultItems(v).to_sse(),
                Err(e) => AppEvent::Error(format!("error in db : {}", e)).to_sse(),
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive-all"),
    )
}

pub async fn stream(
    State(ctx): State<AppContext>,
    Query(params): Query<VaultActionPayload>,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
    let item = vault::Entity::find_by_id(params.vault_id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    let file_path = match item.temp_path {
        Some(v) => PathBuf::from(v),
        None => {
            tracing::warn!(
                temp_path = ?item.temp_path,
                "unable to server from temp_path. switching to  raw file"
            );
            Streaming::resolve_file_path(&item.destination_path)
                .await?
                .0
        }
    };
    let mut sf = ServeFile::new(file_path);

    let resp = sf.try_call(req).await?;

    Ok(resp)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("vault")
        .add("add", post(add))
        .add("all", get(all))
        .add("pause", post(pause))
        .add("resume", post(resume))
        .add("delete", post(delete))
        .add("active", get(active))
        .add("stream", get(stream))
}
