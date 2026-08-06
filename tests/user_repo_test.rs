use komorebi_server::{db::user_repo::UserRepo, models::media::MediaProvider};
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
async fn test_user_repo_crud_and_upsert_operations() {
    let pool = setup_test_db().await;
    let repo = UserRepo::new(pool);

    // 1. Save User (Insert)
    let saved_user = repo
        .save_user(
            "test_username".to_string(),
            Some("https://example.com/avatar.png".to_string()),
            MediaProvider::MAL,
            Some("secret_token_123".to_string()),
        )
        .await
        .expect("failed to save user");
    assert_eq!(saved_user.username, "test_username");
    assert_eq!(saved_user.provider, MediaProvider::MAL);
    assert!(!saved_user.is_sandbox);

    // 2. Fetch User by ID
    let fetched_user = repo
        .fetch_user_by_id(saved_user.id)
        .await
        .expect("failed to fetch user by id")
        .expect("user not found");
    assert_eq!(fetched_user.id, saved_user.id);
    assert_eq!(fetched_user.username, "test_username");
    assert_eq!(fetched_user.provider, MediaProvider::MAL);
    assert_eq!(
        fetched_user.access_token,
        Some("secret_token_123".to_string())
    );

    // 3. Fetch User by Username & Provider
    let fetched_by_name = repo
        .fetch_user_by_username("test_username".to_string(), MediaProvider::MAL)
        .await
        .expect("failed to fetch user by username")
        .expect("user not found by username");
    assert_eq!(fetched_by_name.id, saved_user.id);

    // 4. Upsert User on (username, provider) conflict
    let upserted_user = repo
        .save_user(
            "test_username".to_string(),
            Some("https://example.com/new_avatar.png".to_string()),
            MediaProvider::MAL,
            Some("new_secret_token_456".to_string()),
        )
        .await
        .expect("failed to upsert user");

    // The ID and created_at should remain the same on conflict
    assert_eq!(upserted_user.id, saved_user.id);
    assert_eq!(
        upserted_user.avatar_url,
        Some("https://example.com/new_avatar.png".to_string())
    );
    assert_eq!(
        upserted_user.access_token,
        Some("new_secret_token_456".to_string())
    );

    // 5. Delete User
    repo.delete_user(upserted_user.id)
        .await
        .expect("failed to delete user");

    let deleted_user = repo
        .fetch_user_by_id(upserted_user.id)
        .await
        .expect("query error after delete");
    assert!(deleted_user.is_none());
}

#[tokio::test]
async fn test_fetch_nonexistent_user() {
    let pool = setup_test_db().await;
    let repo = UserRepo::new(pool);

    let result = repo
        .fetch_user_by_id(Uuid::now_v7())
        .await
        .expect("query error");
    assert!(result.is_none());
}
