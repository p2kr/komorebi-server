#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

mod m20260825_072131_vaults;

mod m20260907_064400_vault_sub_items;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20260825_072131_vaults::Migration),
            Box::new(m20260907_064400_vault_sub_items::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
