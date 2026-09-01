use futures::future::join_all;
use loco_rs::prelude::async_trait;
use loco_rs::{
    Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use reqwest::Client;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::broadcast::Sender;
use tokio::sync::broadcast::{self};

use crate::models::events::AppEvent;
#[allow(unused_imports)]
use crate::{controllers, models::_entities::users, workers::downloader::DownloadWorker};
use crate::{
    core::client,
    downloaders::{daemon::start_daemon, manager::DownloadManager},
};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .prefix("/api/v1")
            .add_route(controllers::media_controller::routes())
            .add_route(controllers::user_controller::routes())
            .add_route(controllers::crawler_controller::routes())
            .add_route(controllers::vault_controller::routes()) // controller routes below
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }

    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        let client = client::get_reqwest_client()?;
        ctx.shared_store.insert::<Client>(client.clone());

        // 100 will round off to 128.
        let (tx, _) = broadcast::channel::<AppEvent>(100);
        ctx.shared_store.insert::<Sender<AppEvent>>(tx.clone());

        let download_manager = DownloadManager::new(&ctx.db, client).await?;
        ctx.shared_store
            .insert::<Arc<DownloadManager>>(download_manager.clone());

        // start the download daemon
        start_daemon(ctx.clone(), download_manager, tx);

        Ok(ctx)
    }

    async fn on_shutdown(ctx: &AppContext) {
        // Spawn a watchdog task that enforces a hard process exit after X seconds.
        // If graceful shutdown completes before Xs, the Tokio runtime drops this task automatically.
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::error!("Graceful shutdown timed out after 10 seconds. Forcing process exit.");
            std::process::exit(1);
        });

        let dm = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();
        let engines = dm.get_all_engines();
        let ft = engines.iter().map(|v| v.stop());

        join_all(ft).await;
    }
}
