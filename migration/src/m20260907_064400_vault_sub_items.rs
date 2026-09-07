use loco_rs::schema::*;
use sea_orm_migration::{async_trait::async_trait, prelude::*};

use crate::m20260825_072131_vaults::Vault;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub enum VaultSubItem {
    Table,
    Id,
    VaultId,
    SourcePath,
    DestPath,
    MediaType,
    MediaId,
    Title,
    RawTitle,
    Season,  // volume for manga/novel
    Episode, // chapter for manga/novel
    SourceUrl,
    DownloadType,
    Status,
    TotalBytes,
    Progress,
    SpeedBps,
    EtaSeconds,
    ErrorMsg,
    CreatedAt,
    UpdatedAt,
}

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let mut fk = ForeignKey::create()
            .name("fk_vault_id")
            .from(VaultSubItem::Table, VaultSubItem::VaultId)
            .to(Vault::Table, Vault::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .to_owned();

        let table = Table::create()
            .table(VaultSubItem::Table)
            .if_not_exists()
            .col(pk_uuid(VaultSubItem::Id))
            .col(uuid(VaultSubItem::VaultId)) // Foreign key to `vault`
            .col(string_uniq(VaultSubItem::SourcePath))
            .col(string(VaultSubItem::MediaType).default("ANIME"))
            .col(text_null(VaultSubItem::MediaId))
            .col(string(VaultSubItem::Title))
            .col(string(VaultSubItem::RawTitle))
            .col(string_null(VaultSubItem::Season))
            .col(string_null(VaultSubItem::Episode))
            .col(string(VaultSubItem::SourceUrl))
            .col(string(VaultSubItem::DownloadType).default("MAGNET")) // enum
            .col(string(VaultSubItem::Status).default("PENDING")) // enum
            .col(big_integer(VaultSubItem::TotalBytes).default(0))
            .col(float(VaultSubItem::Progress).default(0.0))
            .col(big_integer(VaultSubItem::SpeedBps).default(0))
            .col(big_integer_null(VaultSubItem::EtaSeconds))
            .col(string_null(VaultSubItem::DestPath))
            .col(string_null(VaultSubItem::ErrorMsg))
            .col(timestamp_with_time_zone_default_now(
                VaultSubItem::CreatedAt,
            ))
            .col(timestamp_with_time_zone_default_now(
                VaultSubItem::UpdatedAt,
            ))
            .foreign_key(&mut fk)
            .to_owned();

        let index1 = Index::create()
            .name("idx_vault_sub_item_vault_id")
            .table(VaultSubItem::Table)
            .col(VaultSubItem::VaultId)
            .to_owned();

        let index2 = Index::create()
            .name("idx_vault_sub_item_status")
            .table(VaultSubItem::Table)
            .col(VaultSubItem::Status)
            .to_owned();

        m.create_table(table).await?;
        m.create_index(index1).await?;
        m.create_index(index2).await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(VaultSubItem::Table).to_owned())
            .await
    }
}
