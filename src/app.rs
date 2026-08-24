use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

use crate::initializers::client;
#[allow(unused_imports)]
use crate::{controllers, models::_entities::users, workers::downloader::DownloadWorker};

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
        Ok(ctx)
    }
}
