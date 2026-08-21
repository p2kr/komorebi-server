use sea_orm_migration::{async_trait::async_trait, prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub enum Users {
    Table,
    Id,
    Username,
    ProviderId,
    AvatarUrl,
    Provider,
    IsSandbox,
    AccessToken,
    Passcode,
    CreatedAt,
    UpdatedAt,
}

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create `users` table
        let table = Table::create()
            .table(Users::Table)
            .if_not_exists()
            .col(pk_uuid(Users::Id))
            .col(string(Users::Username))
            .col(string_null(Users::ProviderId))
            .col(string_null(Users::AvatarUrl))
            .col(string(Users::Provider))
            .col(boolean(Users::IsSandbox).default(true))
            .col(string_null(Users::AccessToken))
            .col(string_null(Users::Passcode))
            .col(timestamp_with_time_zone_default_now(Users::CreatedAt))
            .col(timestamp_with_time_zone_default_now(Users::UpdatedAt))
            .to_owned();

        manager.create_table(table).await?;

        // 2. Create composite UNIQUE index on (username, provider, is_sandbox)
        let index = Index::create()
            .table(Users::Table)
            .name("idx-uniq-users-username-provider-is_sandbox")
            .col(Users::Username)
            .col(Users::Provider)
            .col(Users::IsSandbox)
            .unique()
            .to_owned();

        manager.create_index(index).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}
