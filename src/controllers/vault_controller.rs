use axum::extract::ws::{Message, WebSocketUpgrade};
use loco_rs::prelude::*;
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

use crate::{
    controllers::{fail, success},
    core::vault_path_resolver::get_dest_path,
    downloaders::manager::DownloadManager,
    models::{
        crawler::{CrawlerResult, ParsedTitle},
        media::MediaType,
        vault::{self, VaultDownloadType, VaultItem, VaultItemStatus},
    },
};

// --- JSON Payloads ---
#[derive(Deserialize)]
pub struct VaultActionPayload {
    vault_id: Uuid,
}

#[derive(Deserialize)]
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

    // 1. Create a new VaultItem in the database with PENDING status
    let inserted_item = vault::ActiveModel::from(vault_item).insert(&ctx.db).await?;

    // 2. Fetch the Download Manager
    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    // 3. Delegate to the correct engine
    let err_msg = match manager.get_engine(&download_type) {
        Some(engine) => match engine.add(&inserted_item).await {
            Ok(_) => None,
            Err(e) => Some(e.to_string()),
        },
        None => Some("Download engine not found for this item".into()),
    };

    // If there's an error, update DB to FAILED and return fail() early
    if let Some(msg) = err_msg {
        vault::ActiveModel::from(inserted_item)
            .update_status(VaultItemStatus::FAILED, Some(msg.clone()))
            .update(&ctx.db)
            .await?;

        return fail(StatusCode::INTERNAL_SERVER_ERROR, &msg, None);
    }

    // Success path: Update DB, wake daemon, and return success()
    let resp_model = vault::ActiveModel::from(inserted_item)
        .update_status(VaultItemStatus::DOWNLOADING, None)
        .update(&ctx.db)
        .await?;

    // 4. Wake up the daemon so it starts polling this new download!
    manager.wake_daemon();

    success(resp_model)
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

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    // Route it to the correct engine!
    if let Some(engine) = manager.get_engine(&item.download_type) {
        engine.pause(&params.vault_id).await?;
    }

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

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    // Route it to the correct engine!
    if let Some(engine) = manager.get_engine(&item.download_type) {
        engine.resume(&params.vault_id).await?;
    }

    success(serde_json::json!("Resumed download"))
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

    // Route it to the correct engine!
    if let Some(engine) = manager.get_engine(&item.download_type) {
        engine.delete(&params.vault_id).await?;
    }

    success(serde_json::json!("Deleted/Cancelled download"))
}

// --- WebSockets ---
#[axum::debug_handler]
pub async fn ws(State(ctx): State<AppContext>, ws: WebSocketUpgrade) -> Response {
    // Fetch the broadcast receiver from shared store
    let tx = ctx.shared_store.get::<Sender<Vec<VaultItem>>>().unwrap();
    let mut rx = tx.subscribe();

    ws.on_upgrade(move |mut socket| async move {
        // Stream progress updates to the frontend
        while let Ok(stats) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&stats)
                && socket.send(Message::Text(json.into())).await.is_err()
            {
                break;
            }
        }
    })
}

#[axum::debug_handler]
pub async fn one(
    State(ctx): State<AppContext>,
    Json(params): Json<VaultActionPayload>,
) -> Result<Response> {
    success(
        vault::Entity::find_by_id(params.vault_id)
            .require_one(&ctx.db)
            .await?,
    )
}

#[axum::debug_handler]
pub async fn all(State(ctx): State<AppContext>) -> Result<Response> {
    success(vault::Entity::find().all(&ctx.db).await?)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("vault")
        .add("add", post(add))
        .add("one", post(one))
        .add("all", post(all))
        .add("pause", post(pause))
        .add("resume", post(resume))
        .add("delete", post(delete))
        .add("ws", get(ws)) // WS upgrade MUST be GET. Shows all active items
}
