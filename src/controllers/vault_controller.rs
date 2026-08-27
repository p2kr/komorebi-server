use axum::extract::ws::{Message, WebSocketUpgrade};
use loco_rs::prelude::*;
use reqwest::Url;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

use crate::{
    controllers::success,
    core::constants::VAULT_LOC,
    downloaders::DownloadManager,
    models::{
        crawler::{CrawlerResult, ParsedTitle},
        media::MediaType,
        vault::{self, VaultDownloadType, VaultItem},
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
    todo!()
}

fn get_media_type(_parsed_title: &ParsedTitle) -> Result<MediaType> {
    // Look at file extensions to decide. In torrent it may be in subdirectory.
    todo!()
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
        destination_path: format!("{}/{}/", *VAULT_LOC, vault_id),
        media_type: get_media_type(&params.crawler_result.parsed_title)?,
        media_id: get_media_id(&params.crawler_result.parsed_title)?,
        title: title.clone(),
        source_url: Some(params.crawler_result.link.clone()),
        download_type: download_type.clone(),
        ..Default::default()
    };

    // 1. Create a new VaultItem in the database with PENDING status
    let inserted_item = vault::ActiveModel::from(vault_item).insert(&ctx.db).await?;

    // 2. Fetch the Download Manager
    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    // 3. Delegate to the correct engine
    if let Some(engine) = manager.get_engine(&download_type) {
        engine.add(&inserted_item).await?;
    }

    // 4. Wake up the daemon so it starts polling this new download!
    manager.wake_daemon();

    // Return the inserted item as JSON
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
        .unwrap();

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
        .unwrap();

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
        .unwrap();

    let manager = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();

    // Route it to the correct engine!
    if let Some(engine) = manager.get_engine(&item.download_type) {
        engine.delete(&params.vault_id).await?;
    }

    success(serde_json::json!("Deleted/Cancelled download"))
}

// --- WebSockets ---

pub async fn ws(State(ctx): State<AppContext>, ws: WebSocketUpgrade) -> axum::response::Response {
    // Fetch the broadcast receiver from shared store
    let tx = ctx.shared_store.get::<Sender<Vec<VaultItem>>>().unwrap();
    let mut rx = tx.subscribe();

    ws.on_upgrade(move |mut socket| async move {
        // Stream progress updates to the frontend
        while let Ok(stats) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&stats) {
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("vault")
        .add("add", post(add))
        .add("pause", post(pause))
        .add("resume", post(resume))
        .add("delete", post(delete))
        .add("ws", get(ws)) // WS upgrade MUST be GET. Shows all active items
}
