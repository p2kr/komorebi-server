mod db;
mod handlers;
mod models;
mod utils;

use axum::serve;
use tracing::{error, info};

use crate::utils::{
    SERVER_START_TIME,
    setup::{get_axum_app, get_tokio_listener, init, shutdown_signal},
};

#[tokio::main]
async fn main() {
    init();

    let app = get_axum_app();
    let listener = get_tokio_listener();

    info!("starting application at {:?}", SERVER_START_TIME);

    let serve = serve(listener.await, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    match serve {
        Ok(_) => info!("closing application"),
        Err(_) => error!("failed to start application"),
    };
}
