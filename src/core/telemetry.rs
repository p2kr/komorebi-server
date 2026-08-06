use std::sync::OnceLock;

use tracing::{info, level_filters::LevelFilter};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, registry, util::SubscriberInitExt};

use crate::core::state::get_app_dir;

const LOG_FILE_NAME: &str = "server_logs.log";

static LOGGER_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init_logger() {
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
