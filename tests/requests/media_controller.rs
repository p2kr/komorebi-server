use komorebi_server::{
    app::App,
    models::{_entities::users::ActiveModel, media::MediaProvider},
};
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue};
use serial_test::serial;
use uuid::Uuid;

/// Creates a sandbox MAL user and returns its ID.
async fn seed_user(ctx: &loco_rs::app::AppContext) -> Uuid {
    let user = ActiveModel {
        username: ActiveValue::Set("media_test_user".to_string()),
        provider: ActiveValue::Set(MediaProvider::MAL),
        is_sandbox: ActiveValue::Set(true),
        ..Default::default()
    };
    user.insert(&ctx.db).await.expect("seed user").id
}

#[tokio::test]
#[serial]
async fn can_get_media_controllers() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let payload = serde_json::json!({ "user_id": user_id });

        // Route must exist — 404 would mean the path is wrong.
        let res = request.post("/api/v1/media/anime").json(&payload).await;
        assert_ne!(res.status_code(), 404, "route /api/v1/media/anime should exist");

        let res = request.post("/api/v1/media/manga").json(&payload).await;
        assert_ne!(res.status_code(), 404, "route /api/v1/media/manga should exist");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_anime() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let payload = serde_json::json!({ "user_id": user_id });

        let res = request.post("/api/v1/media/anime").json(&payload).await;
        assert_ne!(res.status_code(), 404, "route /api/v1/media/anime should exist");
        assert_ne!(res.status_code(), 405, "POST should be accepted on /api/v1/media/anime");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_manga() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let payload = serde_json::json!({ "user_id": user_id });

        let res = request.post("/api/v1/media/manga").json(&payload).await;
        assert_ne!(res.status_code(), 404, "route /api/v1/media/manga should exist");
        assert_ne!(res.status_code(), 405, "POST should be accepted on /api/v1/media/manga");
    })
    .await;
}
