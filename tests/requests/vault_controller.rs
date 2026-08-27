use komorebi_server::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_get_vault_controllers() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers").await;
        assert_eq!(res.status_code(), 200);

        // you can assert content like this:
        // assert_eq!(res.text(), "content");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_add() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers/add").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_all() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers/all").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_one() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers/one").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_pause() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers/pause").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_resume() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers/resume").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_cancel() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers/cancel").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_delete() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/vault_controllers/delete").await;
        assert_eq!(res.status_code(), 200);
    })
    .await;
}
