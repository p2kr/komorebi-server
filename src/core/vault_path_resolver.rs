use uuid::Uuid;

use crate::{core::constants::VAULT_LOC, models::vault::VaultItem};

pub fn get_file_path(vault_item: &VaultItem) -> String {
    format!(
        "{}/S{} EP{} {}",
        vault_item.destination_path,
        vault_item.season.clone().unwrap_or_default(),
        vault_item.episode.clone().unwrap_or_default(),
        vault_item.title
    )
}

pub fn get_dest_path(id: &Uuid) -> String {
    format!("{}/{}", *VAULT_LOC, id)
}
