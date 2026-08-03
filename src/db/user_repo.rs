use anyhow::{Result, bail};
use uuid::Uuid;

use crate::models::user::User;

pub async fn fetch_user_by_id(_user_id: Uuid) -> Result<User> {
    bail!("unable to fetch user")
}
