//! Creates auth session User-Agent observations table.

use sea_orm_migration::prelude::*;

/// Stores User-Agent values observed for auth sessions.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SessionUserAgents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SessionUserAgents::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SessionUserAgents::SessionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SessionUserAgents::UserAgent)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SessionUserAgents::FirstSeenAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SessionUserAgents::LastSeenAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("session_user_agents_session_id_fkey")
                            .from(SessionUserAgents::Table, SessionUserAgents::SessionId)
                            .to(Sessions::Table, Sessions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("session_user_agents_session_user_agent_idx")
                    .table(SessionUserAgents::Table)
                    .col(SessionUserAgents::SessionId)
                    .col(SessionUserAgents::UserAgent)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("session_user_agents_session_seen_idx")
                    .table(SessionUserAgents::Table)
                    .col(SessionUserAgents::SessionId)
                    .col(SessionUserAgents::LastSeenAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SessionUserAgents::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum SessionUserAgents {
    Table,
    Id,
    SessionId,
    UserAgent,
    FirstSeenAt,
    LastSeenAt,
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    Id,
}
