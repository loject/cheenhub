#![warn(missing_docs)]
//! Database migrations for CheenHub.

mod m20260505_000001_create_auth_tables;
mod m20260505_000002_create_servers_table;
mod m20260505_000003_create_server_invites_table;
mod m20260505_000004_create_server_members_and_invite_uses_tables;
mod m20260505_000005_create_server_rooms_table;
mod m20260505_000006_create_text_messages_table;
mod m20260512_000007_add_google_oauth_tables;
mod m20260512_000008_rename_o_auth_tables;
mod m20260512_000009_allow_passwordless_users;
mod m20260512_000010_drop_password_hash_not_null;
mod m20260512_000011_create_password_reset_tokens_table;
mod m20260513_000012_add_user_nickname_updated_at;
mod m20260513_000013_create_user_nickname_history_table;
mod m20260513_000014_create_user_password_change_trace_table;
mod m20260513_000015_create_images_and_user_avatars;
mod m20260518_000016_add_server_invite_revoked_at;
mod m20260518_000017_add_server_avatar_image;
mod m20260518_000018_create_server_member_exclusions_table;
mod m20260519_000019_create_server_roles_table;
mod m20260519_000020_create_server_member_roles_table;
mod m20260519_000021_add_text_message_deleted_at;
mod m20260519_000022_add_text_message_deleted_by_user_id;
mod m20260519_000023_create_text_chat_attachments_table;
mod m20260519_000024_create_session_user_agents_table;

pub use sea_orm_migration::prelude::*;

/// Registry for CheenHub database migrations.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260505_000001_create_auth_tables::Migration),
            Box::new(m20260505_000002_create_servers_table::Migration),
            Box::new(m20260505_000003_create_server_invites_table::Migration),
            Box::new(m20260505_000004_create_server_members_and_invite_uses_tables::Migration),
            Box::new(m20260505_000005_create_server_rooms_table::Migration),
            Box::new(m20260505_000006_create_text_messages_table::Migration),
            Box::new(m20260512_000007_add_google_oauth_tables::Migration),
            Box::new(m20260512_000008_rename_o_auth_tables::Migration),
            Box::new(m20260512_000009_allow_passwordless_users::Migration),
            Box::new(m20260512_000010_drop_password_hash_not_null::Migration),
            Box::new(m20260512_000011_create_password_reset_tokens_table::Migration),
            Box::new(m20260513_000012_add_user_nickname_updated_at::Migration),
            Box::new(m20260513_000013_create_user_nickname_history_table::Migration),
            Box::new(m20260513_000014_create_user_password_change_trace_table::Migration),
            Box::new(m20260513_000015_create_images_and_user_avatars::Migration),
            Box::new(m20260518_000016_add_server_invite_revoked_at::Migration),
            Box::new(m20260518_000017_add_server_avatar_image::Migration),
            Box::new(m20260518_000018_create_server_member_exclusions_table::Migration),
            Box::new(m20260519_000019_create_server_roles_table::Migration),
            Box::new(m20260519_000020_create_server_member_roles_table::Migration),
            Box::new(m20260519_000021_add_text_message_deleted_at::Migration),
            Box::new(m20260519_000022_add_text_message_deleted_by_user_id::Migration),
            Box::new(m20260519_000023_create_text_chat_attachments_table::Migration),
            Box::new(m20260519_000024_create_session_user_agents_table::Migration),
        ]
    }
}
