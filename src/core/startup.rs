use std::{sync::LazyLock, time::Instant};

use axum::{Router, ServiceExt, extract::Request, routing::IntoMakeService};
use chrono::Duration;
use dotenvy::dotenv;
use tokio::{net::TcpListener, signal};
use tower::Layer;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use tracing::{error, info};

use crate::{core::telemetry::init_logger, handlers::make_routes};

pub static SERVER_START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

pub fn init() {
    info!("Server initializing {:?}", SERVER_START_TIME);
    init_logger();
    load_env();
}

pub fn cleanup() {
    todo!()
}

/// Loads environment variables from the `.env` file.
fn load_env() {
    match dotenv() {
        Err(e) => {
            error!("failed to load .env file: {}", e);
        }
        Ok(p) => info!("loaded env configs from {:?}", p),
    }
}

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
        Err(_) => format!("up since {:?}", SERVER_START_TIME),
    }
}

pub async fn get_axum_app() -> IntoMakeService<NormalizePath<Router>> {
    let routes = make_routes().await;

    ServiceExt::<Request>::into_make_service(
        NormalizePathLayer::trim_trailing_slash().layer(routes),
    )
}

pub async fn get_tokio_listener() -> TcpListener {
    TcpListener::bind(get_server_port()).await.unwrap()
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            unimplemented!("yet to implement")
        },
        _ = terminate => {
            unimplemented!("yet to implement")
        },
    }
}
