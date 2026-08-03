use anyhow::Result;
use uuid::Uuid;

use crate::models::user::User;

pub async fn fetch_user_by_id(_user_id: Uuid) -> Result<User> {
    Ok(User {
        username: String::from("p2kr"),
        ..Default::default()
    })
}
