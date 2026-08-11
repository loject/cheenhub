//! Атомарные операции приглашений в Postgres.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait, sea_query::LockType,
};
use uuid::Uuid;

use super::AcceptInviteOutcome;
use super::entities::{server_invite_uses, server_invites, server_members};

pub(super) async fn accept_server_invite(
    database: &DatabaseConnection,
    invite_id: &Uuid,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<AcceptInviteOutcome> {
    let transaction = database.begin().await?;
    let Some(invite) = server_invites::Entity::find_by_id(*invite_id)
        .lock(LockType::Update)
        .one(&transaction)
        .await?
    else {
        transaction.rollback().await?;
        return Ok(AcceptInviteOutcome::NotFound);
    };
    if invite
        .expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        transaction.rollback().await?;
        return Ok(AcceptInviteOutcome::Expired);
    }
    if invite.revoked_at.is_some() {
        transaction.rollback().await?;
        return Ok(AcceptInviteOutcome::Revoked);
    }
    if server_members::Entity::find()
        .filter(server_members::Column::ServerId.eq(invite.server_id))
        .filter(server_members::Column::UserId.eq(*user_id))
        .filter(server_members::Column::LeftAt.is_null())
        .one(&transaction)
        .await?
        .is_some()
    {
        transaction.rollback().await?;
        return Ok(AcceptInviteOutcome::AlreadyMember);
    }
    let uses = server_invite_uses::Entity::find()
        .filter(server_invite_uses::Column::InviteId.eq(*invite_id))
        .count(&transaction)
        .await?;
    if invite
        .max_uses
        .and_then(|max_uses| u64::try_from(max_uses).ok())
        .is_some_and(|max_uses| uses >= max_uses)
    {
        transaction.rollback().await?;
        return Ok(AcceptInviteOutcome::Exhausted);
    }

    server_members::ActiveModel {
        id: Set(Uuid::new_v4()),
        server_id: Set(invite.server_id),
        user_id: Set(*user_id),
        joined_at: Set(now),
        left_at: Set(None),
    }
    .insert(&transaction)
    .await?;
    server_invite_uses::ActiveModel {
        id: Set(Uuid::new_v4()),
        invite_id: Set(*invite_id),
        user_id: Set(*user_id),
        used_at: Set(now),
    }
    .insert(&transaction)
    .await?;
    transaction.commit().await?;

    Ok(AcceptInviteOutcome::Accepted)
}
