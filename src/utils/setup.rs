use axum::{Router, ServiceExt, extract::Request, routing::IntoMakeService};
use tokio::{net::TcpListener, signal};
use tower::Layer;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use tracing_subscriber::{
    EnvFilter, fmt::layer, layer::SubscriberExt, registry, util::SubscriberInitExt,
};

use crate::{handlers::make_routes, utils::get_server_port};

pub fn init() {
    registry()
        .with(EnvFilter::new("debug"))
        .with(layer())
        .init();
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
