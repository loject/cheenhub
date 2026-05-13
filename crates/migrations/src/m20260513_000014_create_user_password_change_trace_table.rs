//! Creates user password change trace table.

use sea_orm_migration::prelude::*;

/// Stores successful user password changes.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserPasswordChangeTrace::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserPasswordChangeTrace::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserPasswordChangeTrace::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserPasswordChangeTrace::SessionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserPasswordChangeTrace::ChangedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("user_password_change_trace_user_id_fkey")
                            .from(
                                UserPasswordChangeTrace::Table,
                                UserPasswordChangeTrace::UserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("user_password_change_trace_session_id_fkey")
                            .from(
                                UserPasswordChangeTrace::Table,
                                UserPasswordChangeTrace::SessionId,
                            )
                            .to(Sessions::Table, Sessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("user_password_change_trace_user_changed_at_idx")
                    .table(UserPasswordChangeTrace::Table)
                    .col(UserPasswordChangeTrace::UserId)
                    .col(UserPasswordChangeTrace::ChangedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(UserPasswordChangeTrace::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum UserPasswordChangeTrace {
    Table,
    Id,
    UserId,
    SessionId,
    ChangedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    Id,
}
