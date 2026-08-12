use komorebi_server::{
    core::{AppError, AppState},
    models::{media::MediaProvider, user::User},
    services::user_service::UserService,
};
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

async fn setup_test_db() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("failed to connect to in-memory db");

    let schema_sql = include_str!("../assets/schema.sql");
    sqlx::raw_sql(schema_sql)
        .execute(&pool)
        .await
        .expect("failed to run schema migrations in test");

    pool
}

#[tokio::test]
async fn test_save_sandbox_user_succeeds() {
    let pool = setup_test_db().await;
    let http_client = reqwest::Client::new();
    let state = AppState {
        db: pool,
        http_client,
    };

    let params = User {
        username: "sandbox_user".to_string(),
        avatar_url: None,
        provider: MediaProvider::MAL,
        access_token: None,
        ..Default::default()
    };

    let res = UserService::save_user(&state, params).await;
    assert!(res.is_ok(), "expected save_user to succeed, got {:?}", res);

    let saved = res.unwrap();
    assert_eq!(saved.username, "sandbox_user");
    assert_eq!(saved.provider, MediaProvider::MAL);
    assert!(
        saved.is_sandbox,
        "user without access_token should be sandbox"
    );
}

#[tokio::test]
async fn test_get_user_by_id_returns_not_found_for_missing_user() {
    let pool = setup_test_db().await;
    let http_client = reqwest::Client::new();
    let state = AppState {
        db: pool,
        http_client,
    };

    let missing_id = Uuid::now_v7();
    let res = UserService::get_user_by_id(&state, missing_id).await;
    assert!(res.is_err());

    if let Err(AppError::UserNotFound(id)) = res {
        assert_eq!(id, missing_id);
    } else {
        panic!("expected AppError::UserNotFound, got {:?}", res);
    }
}
