use komorebi_server::{
    app::App,
    models::{_entities::users::ActiveModel, media::MediaProvider},
};
use loco_rs::{hash, testing::prelude::*};
use sea_orm::{ActiveModelTrait, ActiveValue};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_login_with_passcode() {
    request::<App, _, _>(|request, ctx| async move {
        let username = "komorebi_user";
        let passcode = "secret123";
        let hashed = hash::hash_password(passcode).unwrap();

        let user_active = ActiveModel {
            username: ActiveValue::Set(username.to_string()),
            provider: ActiveValue::Set(MediaProvider::MAL),
            passcode: ActiveValue::Set(Some(hashed)),
            is_sandbox: ActiveValue::Set(true),
            ..Default::default()
        };

        let user = user_active
            .insert(&ctx.db)
            .await
            .expect("Failed to insert user");

        // 1. Login with correct passcode
        let login_payload = serde_json::json!({
            "username": username,
            "passcode": passcode,
            "provider": "MAL",
            "is_sandbox": true
        });
        let login_response = request.post("/api/v1/user/login").json(&login_payload).await;
        assert_eq!(login_response.status_code(), 200);

        let logged_in_user: serde_json::Value =
            serde_json::from_str(&login_response.text()).unwrap();
        assert_eq!(logged_in_user["id"], user.id.to_string());
        assert_eq!(logged_in_user["username"], username);
        assert_eq!(logged_in_user["provider"], "MAL");

        // 2. Login with wrong passcode fails
        let wrong_login = serde_json::json!({
            "username": username,
            "passcode": "wrong_passcode",
            "provider": "MAL",
            "is_sandbox": true
        });
        let wrong_response = request.post("/api/v1/user/login").json(&wrong_login).await;
        assert_eq!(wrong_response.status_code(), 401);

        // 3. Login with nonexistent user fails
        let nonexistent_login = serde_json::json!({
            "username": "nonexistent_user",
            "passcode": passcode,
            "provider": "MAL",
            "is_sandbox": true
        });
        let nonexistent_response = request
            .post("/api/v1/user/login")
            .json(&nonexistent_login)
            .await;
        assert_eq!(nonexistent_response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_login_with_empty_passcode() {
    request::<App, _, _>(|request, ctx| async move {
        let username = "sandbox_user";

        let user_active = ActiveModel {
            username: ActiveValue::Set(username.to_string()),
            provider: ActiveValue::Set(MediaProvider::ANILIST),
            passcode: ActiveValue::Set(None),
            is_sandbox: ActiveValue::Set(true),
            ..Default::default()
        };

        let user = user_active
            .insert(&ctx.db)
            .await
            .expect("Failed to insert user");

        // 1. Login with empty passcode
        let login_payload = serde_json::json!({
            "username": username,
            "passcode": "",
            "provider": "ANILIST",
            "is_sandbox": true
        });
        let login_response = request.post("/api/v1/user/login").json(&login_payload).await;
        assert_eq!(login_response.status_code(), 200);

        let logged_in_user: serde_json::Value =
            serde_json::from_str(&login_response.text()).unwrap();
        assert_eq!(logged_in_user["id"], user.id.to_string());

        // 2. Login with null passcode
        let login_null = serde_json::json!({
            "username": username,
            "passcode": null,
            "provider": "ANILIST",
            "is_sandbox": true
        });
        let login_null_response = request.post("/api/v1/user/login").json(&login_null).await;
        assert_eq!(login_null_response.status_code(), 200);

        // 3. Login with wrong passcode should fail
        let wrong_login = serde_json::json!({
            "username": username,
            "passcode": "non_empty_passcode",
            "provider": "ANILIST",
            "is_sandbox": true
        });
        let wrong_response = request.post("/api/v1/user/login").json(&wrong_login).await;
        assert_eq!(wrong_response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_and_delete_user() {
    request::<App, _, _>(|request, ctx| async move {
        let username = "delete_me";
        let user_active = ActiveModel {
            username: ActiveValue::Set(username.to_string()),
            provider: ActiveValue::Set(MediaProvider::MAL),
            is_sandbox: ActiveValue::Set(true),
            ..Default::default()
        };

        let user = user_active
            .insert(&ctx.db)
            .await
            .expect("Failed to insert user");
        let user_id = user.id;

        // Get by ID (POST /api/v1/user/one)
        let one_response = request
            .post("/api/v1/user/one")
            .json(&serde_json::json!({ "user_id": user_id }))
            .await;
        assert_eq!(one_response.status_code(), 200);
        let one_data: serde_json::Value = serde_json::from_str(&one_response.text()).unwrap();
        assert_eq!(one_data["success"], true);
        assert_eq!(one_data["data"]["id"], user_id.to_string());

        // Get all users (POST /api/v1/user/all)
        let all_response = request.post("/api/v1/user/all").await;
        assert_eq!(all_response.status_code(), 200);
        let all_data: serde_json::Value = serde_json::from_str(&all_response.text()).unwrap();
        assert_eq!(all_data["success"], true);
        assert!(!all_data["data"].as_array().unwrap().is_empty());

        // Delete user (POST /api/v1/user/delete)
        let del_response = request
            .post("/api/v1/user/delete")
            .json(&serde_json::json!({ "user_id": user_id }))
            .await;
        assert_eq!(del_response.status_code(), 200);
        let del_data: serde_json::Value = serde_json::from_str(&del_response.text()).unwrap();
        assert_eq!(del_data["success"], true);
        assert_eq!(del_data["data"], user_id.to_string());

        // Verify user is deleted
        let after_del = request
            .post("/api/v1/user/one")
            .json(&serde_json::json!({ "user_id": user_id }))
            .await;
        assert_ne!(after_del.status_code(), 200);
    })
    .await;
}
