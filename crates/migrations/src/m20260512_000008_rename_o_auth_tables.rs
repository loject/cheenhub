//! Renames OAuth tables created with unintended `o_auth_*` names.

use sea_orm_migration::prelude::*;

/// Renames legacy OAuth table names to the names expected by SeaORM entities.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_if_needed(manager, "o_auth_accounts", "oauth_accounts").await?;
        rename_if_needed(manager, "o_auth_states", "oauth_states").await?;
        rename_if_needed(
            manager,
            "o_auth_registration_intents",
            "oauth_registration_intents",
        )
        .await?;
        rename_if_needed(manager, "o_auth_handoffs", "oauth_handoffs").await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_if_needed(manager, "oauth_handoffs", "o_auth_handoffs").await?;
        rename_if_needed(
            manager,
            "oauth_registration_intents",
            "o_auth_registration_intents",
        )
        .await?;
        rename_if_needed(manager, "oauth_states", "o_auth_states").await?;
        rename_if_needed(manager, "oauth_accounts", "o_auth_accounts").await
    }
}

async fn rename_if_needed(
    manager: &SchemaManager<'_>,
    old_name: &'static str,
    new_name: &'static str,
) -> Result<(), DbErr> {
    if manager.has_table(old_name).await? && !manager.has_table(new_name).await? {
        manager
            .rename_table(Table::rename().table(old_name, new_name).to_owned())
            .await?;
    }

    Ok(())
}
