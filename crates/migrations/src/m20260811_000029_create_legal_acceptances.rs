//! Создаёт журнал подтверждений версий юридических документов.

use sea_orm_migration::prelude::*;

/// Миграция журнала юридически значимых подтверждений пользователя.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LegalAcceptances::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LegalAcceptances::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LegalAcceptances::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(LegalAcceptances::DocumentKind)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LegalAcceptances::DocumentVersion)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LegalAcceptances::AcceptanceSource)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LegalAcceptances::AcceptedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_legal_acceptances_user")
                            .from(LegalAcceptances::Table, LegalAcceptances::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_legal_acceptances_user_document_version")
                    .table(LegalAcceptances::Table)
                    .col(LegalAcceptances::UserId)
                    .col(LegalAcceptances::DocumentKind)
                    .col(LegalAcceptances::DocumentVersion)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LegalAcceptances::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LegalAcceptances {
    Table,
    Id,
    UserId,
    DocumentKind,
    DocumentVersion,
    AcceptanceSource,
    AcceptedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
