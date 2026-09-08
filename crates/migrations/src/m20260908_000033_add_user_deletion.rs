//! Поля tombstone в записи пользователя без отдельной таблицы удаления.

use sea_orm_migration::prelude::*;

/// Добавляет сроки удаления, хеш восстановления и время обезличивания в users.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::DeletionRequestedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Users::DeletionRestoreUntil)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Users::DeletionTokenHash)
                            .string_len(64)
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Users::DeletionFinalizedAt)
                            .timestamp_with_time_zone()
                            .null()
                            .check(deletion_state_consistent()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_users_deletion_token_hash")
                    .table(Users::Table)
                    .col(Users::DeletionTokenHash)
                    .unique()
                    .and_where(Expr::col(Users::DeletionTokenHash).is_not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_users_pending_deletion")
                    .table(Users::Table)
                    .col(Users::DeletionRestoreUntil)
                    .and_where(Expr::col(Users::DeletionRequestedAt).is_not_null())
                    .and_where(Expr::col(Users::DeletionFinalizedAt).is_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in [
            "idx_users_deletion_token_hash",
            "idx_users_pending_deletion",
        ] {
            manager
                .drop_index(Index::drop().name(name).table(Users::Table).to_owned())
                .await?;
        }
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::DeletionFinalizedAt)
                    .drop_column(Users::DeletionTokenHash)
                    .drop_column(Users::DeletionRestoreUntil)
                    .drop_column(Users::DeletionRequestedAt)
                    .to_owned(),
            )
            .await
    }
}

fn deletion_state_consistent() -> Condition {
    let active = Condition::all()
        .add(Expr::col(Users::DeletionRequestedAt).is_null())
        .add(Expr::col(Users::DeletionRestoreUntil).is_null())
        .add(Expr::col(Users::DeletionTokenHash).is_null())
        .add(Expr::col(Users::DeletionFinalizedAt).is_null());
    let pending = Condition::all()
        .add(Expr::col(Users::DeletionTokenHash).is_not_null())
        .add(Expr::col(Users::DeletionFinalizedAt).is_null());
    let finalized = Condition::all()
        .add(Expr::col(Users::DeletionTokenHash).is_null())
        .add(Expr::col(Users::DeletionFinalizedAt).is_not_null());
    let deleted = Condition::all()
        .add(Expr::col(Users::DeletionRequestedAt).is_not_null())
        .add(Expr::col(Users::DeletionRestoreUntil).is_not_null())
        .add(Expr::col(Users::DeletionRestoreUntil).gt(Expr::col(Users::DeletionRequestedAt)))
        .add(Condition::any().add(pending).add(finalized));
    Condition::any().add(active).add(deleted)
}

#[derive(DeriveIden)]
enum Users {
    Table,
    DeletionRequestedAt,
    DeletionRestoreUntil,
    DeletionTokenHash,
    DeletionFinalizedAt,
}
