pub mod setup;

use std::{fs::create_dir_all, path::PathBuf, sync::LazyLock, time::Instant};

use chrono::Duration;
use directories::ProjectDirs;
use tracing::info;

const DB_NAME: &str = "server_db.sqlite";
const APP_NAME: &str = "komorebi-server";
const ORG_NAME: &str = "com.github.p2kr";

static SERVER_START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

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

/// using localhost at 8080 by default.
/// TODO: Make it dynamic
pub fn get_server_port() -> &'static str {
    let ip = "127.0.0.1:8080";

    info!("using ip {:?}", &ip);

    ip
}

pub fn get_server_uptime() -> String {
    let elapsed = SERVER_START_TIME.elapsed();

    match Duration::from_std(elapsed) {
        Ok(v) => {
            format!(
                "{}d {:02}h {:02}m {:02}s",
                v.num_days(),
                v.num_hours() % 24,
                v.num_minutes() % 60,
                v.num_seconds() % 60,
            )
        }
        Err(_) => String::from(format!("up since {:?}", SERVER_START_TIME)),
    }
}
