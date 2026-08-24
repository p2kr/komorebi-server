use komorebi_server::adapters::{anilist_client::AniListClient, MediaClient, MediaClientParams};
use komorebi_server::core::ResultExt;
use komorebi_server::models::_entities::users::Model as User;
use uuid::Uuid;

// ─── MediaClientParams::default ──────────────────────────────────────────────

#[test]
fn media_client_params_default_values() {
    let p = MediaClientParams::default();
    assert_eq!(p.limit, Some(50));
    assert_eq!(p.offset, Some(0));
    assert!(p.status.is_none());
    assert!(p.sort.is_none());
    // user_id should be the zero UUID
    assert_eq!(p.user_id, Uuid::default());
}

#[test]
fn media_client_params_type_alias_works() {
    // MedialClientParams is a type alias for MediaClientParams
    let p = komorebi_server::adapters::MedialClientParams::default();
    assert_eq!(p.limit, Some(50));
}

// ─── ResultExt::to_loco_err ──────────────────────────────────────────────────

#[test]
fn result_ext_ok_passes_through() {
    let ok: Result<i32, String> = Ok(42);
    let loco_result = ok.to_loco_err();
    assert!(loco_result.is_ok());
    assert_eq!(loco_result.unwrap(), 42);
}

#[test]
fn result_ext_err_becomes_loco_error() {
    let err: Result<i32, String> = Err("something broke".into());
    let loco_result = err.to_loco_err();
    assert!(loco_result.is_err());
    let err_msg = format!("{}", loco_result.unwrap_err());
    assert!(err_msg.contains("something broke"));
}

#[test]
fn result_ext_works_with_display_error_types() {
    use std::num::ParseIntError;
    let parse_err: Result<i32, ParseIntError> = "not_a_number".parse::<i32>();
    assert!(parse_err.to_loco_err().is_err());
}

// ─── AniListClient::exchange_oauth_token ─────────────────────────────────────

fn make_anilist_client() -> AniListClient {
    let client = reqwest::Client::new();
    let user = User::default();
    AniListClient::new(&client, &user)
}

#[tokio::test]
async fn exchange_oauth_token_code_returned_when_nonempty() {
    let ac = make_anilist_client();
    let token = ac.exchange_oauth_token("my_code", "").await.unwrap();
    assert_eq!(token, "my_code");
}

#[tokio::test]
async fn exchange_oauth_token_verifier_returned_when_code_empty() {
    let ac = make_anilist_client();
    let token = ac.exchange_oauth_token("", "my_verifier").await.unwrap();
    assert_eq!(token, "my_verifier");
}

#[tokio::test]
async fn exchange_oauth_token_both_empty_is_err() {
    let ac = make_anilist_client();
    let result = ac.exchange_oauth_token("", "").await;
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("cannot be empty"));
}

#[tokio::test]
async fn exchange_oauth_token_code_takes_priority_over_verifier() {
    let ac = make_anilist_client();
    // When both are present, code wins
    let token = ac
        .exchange_oauth_token("the_code", "the_verifier")
        .await
        .unwrap();
    assert_eq!(token, "the_code");
}
