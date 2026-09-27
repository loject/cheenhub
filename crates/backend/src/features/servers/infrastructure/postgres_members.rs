//! Postgres-backed membership and exclusion storage helpers.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use super::entities::{server_member_exclusions, server_members};
use crate::features::servers::domain::{ServerMember, ServerMemberExclusion};

pub(super) async fn insert_server_member(
    database: &DatabaseConnection,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<ServerMember> {
    let model = server_members::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(*server_id),
        user_id: Set(*user_id),
        joined_at: Set(Utc::now()),
        left_at: Set(None),
    }
    .insert(database)
    .await?;

    Ok(model.into())
}

pub(super) async fn find_active_server_member(
    database: &DatabaseConnection,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<Option<ServerMember>> {
    Ok(server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(*server_id))
        .filter(server_members::Column::UserId.eq(*user_id))
        .filter(server_members::Column::LeftAt.is_null())
        .one(database)
        .await?
        .map(Into::into))
}

pub(super) async fn list_active_server_members(
    database: &DatabaseConnection,
    server_id: &Uuid,
) -> anyhow::Result<Vec<ServerMember>> {
    Ok(server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(*server_id))
        .filter(server_members::Column::LeftAt.is_null())
        .order_by_asc(server_members::Column::JoinedAt)
        .all(database)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

pub(super) async fn leave_server(
    database: &DatabaseConnection,
    server_id: &Uuid,
    user_id: &Uuid,
) -> anyhow::Result<()> {
    let Some(member) = server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(*server_id))
        .filter(server_members::Column::UserId.eq(*user_id))
        .filter(server_members::Column::LeftAt.is_null())
        .one(database)
        .await?
    else {
        return Ok(());
    };
    let mut member = member.into_active_model();
    member.left_at = Set(Some(Utc::now()));
    member.update(database).await?;

    Ok(())
}

pub(super) async fn insert_server_member_exclusion(
    database: &DatabaseConnection,
    server_id: &Uuid,
    user_id: &Uuid,
    initiator_user_id: &Uuid,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<ServerMemberExclusion> {
    let model = server_member_exclusions::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(*server_id),
        user_id: Set(*user_id),
        initiator_user_id: Set(*initiator_user_id),
        expires_at: Set(expires_at),
        created_at: Set(Utc::now()),
    }
    .insert(database)
    .await?;

    Ok(model.into())
}

pub(super) async fn find_active_server_member_exclusion(
    database: &DatabaseConnection,
    server_id: &Uuid,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<ServerMemberExclusion>> {
    Ok(server_member_exclusions::Entity::find()
        .filter(server_member_exclusions::Column::ServerId.eq(*server_id))
        .filter(server_member_exclusions::Column::UserId.eq(*user_id))
        .filter(server_member_exclusions::Column::ExpiresAt.gt(now))
        .order_by_desc(server_member_exclusions::Column::ExpiresAt)
        .one(database)
        .await?
        .map(Into::into))
}
