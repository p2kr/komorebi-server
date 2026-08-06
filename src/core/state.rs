use std::{fs::create_dir_all, path::PathBuf, time};

use directories::ProjectDirs;
use sqlx::SqlitePool;
use tracing::info;

use crate::db::init_db;

const DB_NAME: &str = "server_db.sqlite";
const APP_NAME: &str = "komorebi-server";
const ORG_NAME: &str = "com.github.p2kr";

pub fn get_app_dir() -> PathBuf {
    let app_dir = match ProjectDirs::from("", ORG_NAME, APP_NAME) {
        Some(dir) => dir.config_dir().to_path_buf(),
        None => {
            let fallback_dir = PathBuf::from("/app_config/");
            create_dir_all(&fallback_dir).ok();
            fallback_dir
        }
    };

    info!(
        "app dir initialized at {}",
        app_dir.to_str().unwrap_or("<ERR>")
    );

    app_dir
}

pub fn get_db_path() -> PathBuf {
    get_app_dir().join(DB_NAME)
}

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub http_client: reqwest::Client,
}

pub async fn load_app_state() -> AppState {
    let http_client = reqwest::Client::builder()
        .timeout(time::Duration::from_secs(15))
        .build()
        .expect("failed to create http client pool");

    AppState {
        db: init_db().await,
        http_client,
    }
}
