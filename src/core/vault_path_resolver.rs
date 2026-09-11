use uuid::Uuid;

use crate::{
    core::constants::VAULT_LOC,
    models::{vault::VaultItem, vault_sub_item::VaultSubItem},
};

pub fn get_file_path(vault_item: &VaultItem) -> String {
    format!("{}/{}", vault_item.dest_path, vault_item.title)
}

pub fn get_sub_item_file_path(sub_item: &VaultSubItem) -> String {
    format!(
        "{}/S{} EP{} {}",
        sub_item.source_path,
        sub_item.season.clone().unwrap_or_default(),
        sub_item.episode.clone().unwrap_or_default(),
        sub_item.title
    )
}

pub fn get_dest_path(id: &Uuid) -> String {
    format!("{}/{}", *VAULT_LOC, id)
}
