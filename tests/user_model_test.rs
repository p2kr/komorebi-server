use komorebi_server::models::{media::MediaProvider, user::User};

#[test]
fn test_user_is_sandbox_derived_from_access_token() {
    let user_with_token = User::new(
        "test_user".to_string(),
        None,
        Some(MediaProvider::MAL),
        Some("token123".to_string()),
    );
    assert!(!user_with_token.is_sandbox);

    let user_without_token = User::new(
        "test_user".to_string(),
        None,
        Some(MediaProvider::MAL),
        None,
    );
    assert!(user_without_token.is_sandbox);

    let default_user = User::default();
    assert!(default_user.is_sandbox);
}
