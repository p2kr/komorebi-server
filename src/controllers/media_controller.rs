use loco_rs::prelude::*;
use reqwest::Client;

use crate::{
    adapters::MediaClientParams,
    controllers::success,
    models::users::{self},
};

#[debug_handler]
pub async fn get_user_anime(
    State(ctx): State<AppContext>,
    Json(params): Json<MediaClientParams>,
) -> Result<Response> {
    // get the user
    let user = users::Entity::find_by_id(params.user_id)
        .require_one(&ctx.db)
        .await?;

    // get the user's anime
    let media_client = user
        .provider
        .new_client(&ctx.shared_store.get::<Client>().unwrap(), &user);

    let anime_list = media_client.get_anime_list(&params).await?;

    success(anime_list)
}

#[debug_handler]
pub async fn get_user_manga(
    State(ctx): State<AppContext>,
    Json(params): Json<MediaClientParams>,
) -> Result<Response> {
    // get the user
    let user = users::Entity::find_by_id(params.user_id)
        .require_one(&ctx.db)
        .await?;

    // get the user's anime
    let media_client = user
        .provider
        .new_client(&ctx.shared_store.get::<Client>().unwrap(), &user);

    let manga_list = media_client.get_manga_list(&params).await?;

    success(manga_list)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/media")
        .add("/anime", post(get_user_anime))
        .add("/manga", post(get_user_manga))
}
