use komorebi_server::{
    app::App,
    models::{
        _entities::users,
        crawler::CrawlerResult,
        media::{MediaProvider, MediaType},
        vault::{self, VaultDownloadType, VaultItem, VaultItemStatus},
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue};
use serial_test::serial;
use uuid::Uuid;

async fn seed_user(ctx: &loco_rs::app::AppContext) -> Uuid {
    let user = users::ActiveModel {
        username: ActiveValue::Set("vault_test_user".into()),
        provider: ActiveValue::Set(MediaProvider::MAL),
        is_sandbox: ActiveValue::Set(true),
        ..Default::default()
    };
    user.insert(&ctx.db).await.expect("seed user").id
}

async fn seed_vault_item(ctx: &loco_rs::app::AppContext, user_id: Uuid, title: &str) -> VaultItem {
    let vault_id = Uuid::now_v7();
    let item = VaultItem {
        id: vault_id,
        user_id,
        destination_path: format!("/tmp/vault_{}", vault_id),
        media_type: Some(MediaType::Anime),
        media_id: "123".into(),
        title: title.to_string(),
        raw_title: format!("[SubGroup] {} - 01 [1080p]", title),
        source_url: format!("http://example.com/{}.mp4", title),
        download_type: VaultDownloadType::DIRECT,
        status: VaultItemStatus::DOWNLOADING,
        ..Default::default()
    };
    vault::ActiveModel::from(item)
        .insert(&ctx.db)
        .await
        .expect("seed vault item")
}

#[tokio::test]
#[serial]
async fn can_get_vault_controllers() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.post("/api/v1/vault/all").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_add() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let mut crawler_result = CrawlerResult {
            title: "[SubGroup] Test Anime - 01 [1080p].mp4".into(),
            link: "http://127.0.0.1:9999/test.mp4".into(),
            source: "Nyaa".into(),
            category: MediaType::Anime,
            ..Default::default()
        };
        crawler_result
            .parsed_title
            .title
            .insert("Test Anime".into());
        crawler_result.parsed_title.episode.insert("01".into());
        crawler_result.parsed_title.season.insert("1".into());
        crawler_result
            .parsed_title
            .file_extension
            .insert("mp4".into());

        let payload = serde_json::json!({
            "user_id": user_id,
            "crawler_result": crawler_result,
        });

        let res = request.post("/api/v1/vault/add").json(&payload).await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["title"], "Test Anime");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_all() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let seeded = seed_vault_item(&ctx, user_id, "Test Anime All").await;

        let res = request.post("/api/v1/vault/all").await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(body["success"], true);
        assert!(body["data"].is_array());
        let items = body["data"].as_array().unwrap();
        assert!(items.iter().any(|i| i["id"] == seeded.id.to_string()));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_one() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let seeded = seed_vault_item(&ctx, user_id, "Test Anime One").await;

        let payload = serde_json::json!({ "vault_id": seeded.id });
        let res = request.post("/api/v1/vault/one").json(&payload).await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["id"], seeded.id.to_string());
        assert_eq!(body["data"]["title"], "Test Anime One");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_pause() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let seeded = seed_vault_item(&ctx, user_id, "Test Anime Pause").await;

        let payload = serde_json::json!({ "vault_id": seeded.id });
        let res = request.post("/api/v1/vault/pause").json(&payload).await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(body["success"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_resume() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let seeded = seed_vault_item(&ctx, user_id, "Test Anime Resume").await;

        let payload = serde_json::json!({ "vault_id": seeded.id });
        let res = request.post("/api/v1/vault/resume").json(&payload).await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(body["success"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_cancel() {
    request::<App, _, _>(|request, _ctx| async move {
        let non_existent_id = Uuid::now_v7();
        let payload = serde_json::json!({ "vault_id": non_existent_id });
        let res = request.post("/api/v1/vault/delete").json(&payload).await;
        assert_eq!(res.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_delete() {
    request::<App, _, _>(|request, ctx| async move {
        let user_id = seed_user(&ctx).await;
        let seeded = seed_vault_item(&ctx, user_id, "Test Anime Delete").await;

        let payload = serde_json::json!({ "vault_id": seeded.id });
        let res = request.post("/api/v1/vault/delete").json(&payload).await;
        assert_eq!(res.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert_eq!(body["success"], true);
    })
    .await;
}
