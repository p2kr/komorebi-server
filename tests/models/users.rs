use komorebi_server::{
    app::App,
    models::{
        _entities::users::{self, ActiveModel},
        media::MediaProvider,
    },
};
use loco_rs::{hash, testing::prelude::*};
use sea_orm::{ActiveModelTrait, ActiveValue};
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
async fn test_user_model_crud_and_passcode() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");

    let username = "tester";
    let plain_passcode = "pass123";
    let hashed = hash::hash_password(plain_passcode).unwrap();

    let user_active = ActiveModel {
        username: ActiveValue::Set(username.to_string()),
        provider: ActiveValue::Set(MediaProvider::MAL),
        passcode: ActiveValue::Set(Some(hashed)),
        is_sandbox: ActiveValue::Set(true),
        ..Default::default()
    };

    let user = user_active
        .insert(&boot.app_context.db)
        .await
        .expect("Failed to insert user");

    assert_eq!(user.username, username);
    assert_ne!(user.id, Uuid::nil());
    assert!(user.verify_passcode(Some(plain_passcode)));
    assert!(!user.verify_passcode(Some("wrong_pass")));
    assert!(!user.verify_passcode(None));
    assert!(!user.verify_passcode(Some("")));

    // Test find_by_id
    let found_by_id = users::Model::find_by_id(&boot.app_context.db, user.id)
        .await
        .expect("User should be found by id");
    assert_eq!(found_by_id.id, user.id);

    // Test find_by_username_and_provider_and_sandbox
    let found_by_prov = users::Model::find_by_username_and_provider_and_sandbox(
        &boot.app_context.db,
        username,
        MediaProvider::MAL,
        true,
    )
    .await
    .expect("User should be found by username and provider");
    assert_eq!(found_by_prov.id, user.id);

    // Test get_all_users
    let all_users = users::Model::get_all_users(&boot.app_context.db)
        .await
        .expect("Failed to get all users");
    assert!(!all_users.is_empty());

    // Test delete_user
    users::Model::delete_user(&boot.app_context.db, user.id)
        .await
        .expect("Failed to delete user");

    let deleted = users::Model::find_by_id(&boot.app_context.db, user.id).await;
    assert!(deleted.is_err());
}

#[tokio::test]
#[serial]
async fn test_empty_passcode_validation() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");

    let username = "empty_passcode_user";

    let user_active = ActiveModel {
        username: ActiveValue::Set(username.to_string()),
        provider: ActiveValue::Set(MediaProvider::ANILIST),
        passcode: ActiveValue::Set(None),
        is_sandbox: ActiveValue::Set(true),
        ..Default::default()
    };

    let user = user_active
        .insert(&boot.app_context.db)
        .await
        .expect("Failed to insert user");

    assert!(user.verify_passcode(None));
    assert!(user.verify_passcode(Some("")));
    assert!(!user.verify_passcode(Some("any_pass")));
}

#[tokio::test]
#[serial]
async fn test_plain_passcode_branch() {
    // When a passcode is stored as plain text (not argon2-hashed),
    // verify_passcode falls back to a direct string comparison.
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");

    let plain = "plain_secret";
    let user_active = ActiveModel {
        username: ActiveValue::Set("plain_passcode_user".to_string()),
        provider: ActiveValue::Set(MediaProvider::MAL),
        passcode: ActiveValue::Set(Some(plain.to_string())), // stored unencrypted
        is_sandbox: ActiveValue::Set(true),
        ..Default::default()
    };
    let user = user_active
        .insert(&boot.app_context.db)
        .await
        .expect("insert");

    // Correct plain passcode matches
    assert!(user.verify_passcode(Some(plain)));
    // Wrong passcode does not match
    assert!(!user.verify_passcode(Some("wrong")));
    // Empty input does not match a non-empty stored value
    assert!(!user.verify_passcode(Some("")));
    assert!(!user.verify_passcode(None));
}

#[tokio::test]
#[serial]
async fn test_save_user_upsert_updates_token() {
    let boot = boot_test::<App>()
        .await
        .expect("Failed to boot test application");

    let username = "upsert_test_user";

    // First insert
    let u1 = users::Model {
        username: username.to_string(),
        provider: MediaProvider::MAL,
        is_sandbox: false,
        access_token: Some("token_v1".to_string()),
        ..Default::default()
    };
    let saved1 = users::Model::save_user(&boot.app_context.db, u1)
        .await
        .expect("first save");

    // Second upsert with an updated token — same (username, provider, is_sandbox) key
    let u2 = users::Model {
        id: saved1.id,
        username: username.to_string(),
        provider: MediaProvider::MAL,
        is_sandbox: false,
        access_token: Some("token_v2".to_string()),
        ..Default::default()
    };
    let saved2 = users::Model::save_user(&boot.app_context.db, u2)
        .await
        .expect("second save");

    // The access_token should have been updated via ON CONFLICT … DO UPDATE
    assert_eq!(saved2.access_token, Some("token_v2".to_string()));
    assert_eq!(saved2.username, username);
}
