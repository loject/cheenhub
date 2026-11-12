//! PostgreSQL-хранение настроек приватности и направленных блокировок.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
    sea_query::{Expr, OnConflict},
};
use uuid::Uuid;

use super::entities::{self as friendships, dm_messages, user_blocks, user_privacy_settings};
use super::postgres::PostgresSocialStore;
use super::privacy_store::PrivacyStore;
use crate::features::social::domain::{
    IncomingContactRule, PrivacySettings, UserBlock, ordered_pair,
};

#[async_trait]
impl PrivacyStore for PostgresSocialStore {
    async fn privacy_settings(&self, user_id: &Uuid) -> anyhow::Result<PrivacySettings> {
        let Some(row) = user_privacy_settings::Entity::find_by_id(*user_id)
            .one(&self.database)
            .await?
        else {
            return Ok(PrivacySettings::default());
        };
        let direct_messages_friends_only = match row.direct_messages.as_str() {
            "everyone" => false,
            "friends" => true,
            other => anyhow::bail!("unknown direct message privacy rule: {other}"),
        };
        let direct_calls = IncomingContactRule::from_str(&row.direct_calls).ok_or_else(|| {
            anyhow::anyhow!("unknown direct call privacy rule: {}", row.direct_calls)
        })?;
        let voice_room_invites = IncomingContactRule::from_str(&row.voice_room_invites)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown room invite privacy rule: {}",
                    row.voice_room_invites
                )
            })?;
        Ok(PrivacySettings {
            direct_messages_friends_only,
            direct_calls,
            voice_room_invites,
            direct_messages_restricted_since: row.direct_messages_restricted_since,
        })
    }

    async fn save_privacy_settings(
        &self,
        user_id: &Uuid,
        settings: PrivacySettings,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let direct_messages = if settings.direct_messages_friends_only {
            "friends"
        } else {
            "everyone"
        };
        user_privacy_settings::Entity::insert(user_privacy_settings::ActiveModel {
            user_id: Set(*user_id),
            direct_messages: Set(direct_messages.to_owned()),
            direct_calls: Set(settings.direct_calls.as_str().to_owned()),
            voice_room_invites: Set(settings.voice_room_invites.as_str().to_owned()),
            direct_messages_restricted_since: Set(settings.direct_messages_restricted_since),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::column(user_privacy_settings::Column::UserId)
                .update_columns([
                    user_privacy_settings::Column::DirectMessages,
                    user_privacy_settings::Column::DirectCalls,
                    user_privacy_settings::Column::VoiceRoomInvites,
                    user_privacy_settings::Column::DirectMessagesRestrictedSince,
                    user_privacy_settings::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(&self.database)
        .await?;
        Ok(())
    }

    async fn user_blocks_between(
        &self,
        left: &Uuid,
        right: &Uuid,
    ) -> anyhow::Result<Vec<UserBlock>> {
        Ok(user_blocks::Entity::find()
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(user_blocks::Column::BlockerUserId.eq(*left))
                            .add(user_blocks::Column::BlockedUserId.eq(*right)),
                    )
                    .add(
                        Condition::all()
                            .add(user_blocks::Column::BlockerUserId.eq(*right))
                            .add(user_blocks::Column::BlockedUserId.eq(*left)),
                    ),
            )
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn active_user_blocks(&self, user_id: &Uuid) -> anyhow::Result<Vec<UserBlock>> {
        Ok(user_blocks::Entity::find()
            .filter(user_blocks::Column::BlockerUserId.eq(*user_id))
            .filter(user_blocks::Column::UnblockedAt.is_null())
            .order_by_desc(user_blocks::Column::BlockedAt)
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn block_user(
        &self,
        blocker: &Uuid,
        blocked: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<UserBlock> {
        let transaction = self.database.begin().await?;
        let block = user_blocks::Entity::insert(user_blocks::ActiveModel {
            id: Set(Uuid::new_v4()),
            blocker_user_id: Set(*blocker),
            blocked_user_id: Set(*blocked),
            blocked_at: Set(now),
            unblocked_at: Set(None),
        })
        .on_conflict(
            OnConflict::columns([
                user_blocks::Column::BlockerUserId,
                user_blocks::Column::BlockedUserId,
            ])
            .update_columns([
                user_blocks::Column::BlockedAt,
                user_blocks::Column::UnblockedAt,
            ])
            .to_owned(),
        )
        .exec_with_returning(&transaction)
        .await?;
        let (low, high) = ordered_pair(*blocker, *blocked);
        friendships::Entity::update_many()
            .col_expr(friendships::Column::Status, Expr::value("cancelled"))
            .col_expr(friendships::Column::UpdatedAt, Expr::value(now))
            .filter(friendships::Column::UserLowId.eq(low))
            .filter(friendships::Column::UserHighId.eq(high))
            .filter(friendships::Column::Status.is_in(["accepted", "pending"]))
            .exec(&transaction)
            .await?;
        transaction.commit().await?;
        Ok(block.into())
    }

    async fn unblock_user(
        &self,
        blocker: &Uuid,
        blocked: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        let updated = user_blocks::Entity::update_many()
            .col_expr(user_blocks::Column::UnblockedAt, Expr::value(now))
            .filter(user_blocks::Column::BlockerUserId.eq(*blocker))
            .filter(user_blocks::Column::BlockedUserId.eq(*blocked))
            .filter(user_blocks::Column::UnblockedAt.is_null())
            .exec(&self.database)
            .await?;
        Ok(updated.rows_affected > 0)
    }

    async fn has_dm_message_after(
        &self,
        conversation_id: &Uuid,
        author_id: &Uuid,
        after: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        Ok(dm_messages::Entity::find()
            .filter(dm_messages::Column::ConversationId.eq(*conversation_id))
            .filter(dm_messages::Column::SenderUserId.eq(*author_id))
            .filter(dm_messages::Column::CreatedAt.gt(after))
            .one(&self.database)
            .await?
            .is_some())
    }
}

impl From<user_blocks::Model> for UserBlock {
    fn from(row: user_blocks::Model) -> Self {
        Self {
            id: row.id,
            blocker_user_id: row.blocker_user_id,
            blocked_user_id: row.blocked_user_id,
            blocked_at: row.blocked_at,
            unblocked_at: row.unblocked_at,
        }
    }
}
