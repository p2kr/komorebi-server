use loco_rs::schema::*;
use sea_orm_migration::{async_trait::async_trait, prelude::*};

use crate::m20220101_000001_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Vault {
    Table,
    Id,
    UserId,
    DestinationPath,
    MediaType,
    MediaId,
    Title,
    RawTitle,
    Season,
    Episode,
    SourceUrl,
    DownloadType,
    Status,
    TotalBytes,
    DownloadedBytes,
    Progress,
    SpeedBps,
    EtaSeconds,
    TempPath,
    ErrorMsg,
    CreatedAt,
    UpdatedAt,
}

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let mut fk = ForeignKey::create()
            .name("fk_vault_user_id")
            .from(Vault::Table, Vault::UserId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .to_owned();

        let table = Table::create()
            .table(Vault::Table)
            .if_not_exists()
            .col(pk_uuid(Vault::Id))
            .col(uuid(Vault::UserId)) // Foreign key to `users`
            .col(string_uniq(Vault::DestinationPath))
            .col(string(Vault::MediaType).default("ANIME"))
            .col(text_null(Vault::MediaId))
            .col(string(Vault::Title))
            .col(string(Vault::RawTitle))
            .col(string_null(Vault::Season))
            .col(string_null(Vault::Episode))
            .col(string(Vault::SourceUrl))
            .col(string(Vault::DownloadType).default("MAGNET")) // enum
            .col(string(Vault::Status).default("PENDING")) // enum
            .col(big_integer(Vault::TotalBytes).default(0))
            .col(big_integer(Vault::DownloadedBytes).default(0))
            .col(float(Vault::Progress).default(0.0))
            .col(big_integer(Vault::SpeedBps).default(0))
            .col(big_integer_null(Vault::EtaSeconds))
            .col(string_null(Vault::TempPath))
            .col(string_null(Vault::ErrorMsg))
            .col(timestamp_with_time_zone_default_now(Vault::CreatedAt))
            .col(timestamp_with_time_zone_default_now(Vault::UpdatedAt))
            .foreign_key(&mut fk)
            .to_owned();

        let index1 = Index::create()
            .name("idx_vault_user_id")
            .table(Vault::Table)
            .col(Vault::UserId)
            .to_owned();

        let index2 = Index::create()
            .name("idx_vault_status")
            .table(Vault::Table)
            .col(Vault::Status)
            .to_owned();

        m.create_table(table).await?;
        m.create_index(index1).await?;
        m.create_index(index2).await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Vault::Table).to_owned())
            .await
    }
}
