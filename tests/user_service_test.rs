use komorebi_server::{
    core::{AppError, AppState},
    db::init_db,
    models::media::MediaProvider,
    services::user_service::{Params, UserService},
};

#[tokio::test]
async fn test_save_user_fails_if_user_does_not_exist_on_provider() {
    let pool = init_db().await;
    let http_client = reqwest::Client::new();
    let state = AppState {
        db: pool,
        http_client,
    };

    let params = Params {
        username: "non_existent_random_user_9999999999".to_string(),
        avatar_url: None,
        provider: MediaProvider::MAL,
        access_token: None,
    };

    let res = UserService::save_user(&state, params).await;
    assert!(res.is_err());

    if let Err(AppError::UpstreamApi { provider, .. }) = res {
        assert_eq!(provider, "MAL");
    } else {
        panic!("expected AppError::UpstreamApi, got {:?}", res);
    }
}
