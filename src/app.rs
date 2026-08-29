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
use std::{path::Path, sync::Arc, time::Duration};
use tokio::{
    sync::broadcast::{self},
    time::timeout,
};

#[allow(unused_imports)]
use crate::{controllers, models::_entities::users, workers::downloader::DownloadWorker};
use crate::{
    downloaders::{daemon::start_daemon, manager::DownloadManager},
    initializers::client,
    models::vault::VaultItem,
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
            .prefix("/api/v1") // controller routes below
            .add_route(controllers::media_controller::routes())
            .add_route(controllers::user_controller::routes())
            .add_route(controllers::crawler_controller::routes())
            .add_route(controllers::vault_controller::routes())
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
        let client = client::get_reqwest_client();
        ctx.shared_store.insert(client);

        // 100 will round off to 128.
        let (ws, _) = broadcast::channel::<Vec<VaultItem>>(100);
        ctx.shared_store.insert(ws.clone());

        let download_manager = DownloadManager::new(&ctx.db).await?;
        ctx.shared_store.insert(download_manager.clone());

        // start the download daemon
        start_daemon(ctx.clone(), download_manager, ws);

        Ok(ctx)
    }

    async fn on_shutdown(ctx: &AppContext) {
        let dm = ctx.shared_store.get::<Arc<DownloadManager>>().unwrap();
        for engine in dm.get_all_engines() {
            // TODO: Use join_all(...)
            // It's fine for now since only 1 engine has stop implemented.
            timeout(Duration::from_secs(5), engine.stop())
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "Warning: Engine [{}] took longer than 5 seconds to stop",
                        engine
                    )
                });
        }
    }
}
