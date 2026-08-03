use std::sync::OnceLock;

use axum::{Router, ServiceExt, extract::Request, routing::IntoMakeService};
use dotenvy::dotenv;
use tokio::{net::TcpListener, signal};
use tower::Layer;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, registry, util::SubscriberInitExt};

use crate::{
    handlers::make_routes,
    utils::{LOG_FILE_NAME, get_app_dir, get_server_port},
};

static LOGGER_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init() {
    init_logger();
    load_env();
}

fn init_logger() {
    let file_appender = rolling::daily(get_app_dir(), LOG_FILE_NAME);

    let (nb, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer().json().with_writer(nb).with_ansi(false);

    registry()
        .with(file_layer)
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .with(fmt::layer())
        .init();

    let _ = LOGGER_GUARD.set(guard);

    info!("logger initialized at {:?}", get_app_dir());
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

pub fn get_axum_app() -> IntoMakeService<NormalizePath<Router>> {
    let routes = make_routes();

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
