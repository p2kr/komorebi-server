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
    let found_by_prov =
        users::Model::find_by_username_and_provider_and_sandbox(
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
