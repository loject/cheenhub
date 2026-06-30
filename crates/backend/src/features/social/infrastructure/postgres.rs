//! Postgres-хранилище друзей и личных сообщений.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::features::social::domain::{
    DmConversation, DmMessage, Friendship, FriendshipStatus, ordered_pair,
};
use crate::features::social::infrastructure::entities::{
    self as friendships, dm_conversations, dm_messages,
};
use crate::features::social::infrastructure::{DM_HISTORY_LIMIT, DmMessagePage, SocialStore};

/// Postgres-хранилище социальных данных.
pub(crate) struct PostgresSocialStore {
    database: DatabaseConnection,
}

impl PostgresSocialStore {
    /// Создает Postgres-хранилище социальных данных.
    pub(crate) fn new(database: DatabaseConnection) -> Self {
        Self { database }
    }
}

#[async_trait]
impl SocialStore for PostgresSocialStore {
    async fn friendship_between(
        &self,
        left_user_id: &Uuid,
        right_user_id: &Uuid,
    ) -> anyhow::Result<Option<Friendship>> {
        let (user_low_id, user_high_id) = ordered_pair(*left_user_id, *right_user_id);
        Ok(friendships::Entity::find()
            .filter(friendships::Column::UserLowId.eq(user_low_id))
            .filter(friendships::Column::UserHighId.eq(user_high_id))
            .one(&self.database)
            .await?
            .map(try_friendship)
            .transpose()?)
    }

    async fn friendship_by_id(&self, friendship_id: &Uuid) -> anyhow::Result<Option<Friendship>> {
        Ok(friendships::Entity::find_by_id(*friendship_id)
            .one(&self.database)
            .await?
            .map(try_friendship)
            .transpose()?)
    }

    async fn upsert_friend_request(
        &self,
        requester_user_id: &Uuid,
        recipient_user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Friendship> {
        let (user_low_id, user_high_id) = ordered_pair(*requester_user_id, *recipient_user_id);
        if let Some(row) = friendships::Entity::find()
            .filter(friendships::Column::UserLowId.eq(user_low_id))
            .filter(friendships::Column::UserHighId.eq(user_high_id))
            .one(&self.database)
            .await?
        {
            let mut active = row.into_active_model();
            active.requester_user_id = Set(*requester_user_id);
            active.recipient_user_id = Set(*recipient_user_id);
            active.status = Set(FriendshipStatus::Pending.as_str().to_owned());
            active.updated_at = Set(now);
            return try_friendship(active.update(&self.database).await?);
        }

        try_friendship(
            friendships::ActiveModel {
                id: Set(Uuid::new_v4()),
                requester_user_id: Set(*requester_user_id),
                recipient_user_id: Set(*recipient_user_id),
                user_low_id: Set(user_low_id),
                user_high_id: Set(user_high_id),
                status: Set(FriendshipStatus::Pending.as_str().to_owned()),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&self.database)
            .await?,
        )
    }

    async fn update_friendship_status(
        &self,
        friendship_id: &Uuid,
        status: FriendshipStatus,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Friendship>> {
        let Some(row) = friendships::Entity::find_by_id(*friendship_id)
            .one(&self.database)
            .await?
        else {
            return Ok(None);
        };
        let mut active = row.into_active_model();
        active.status = Set(status.as_str().to_owned());
        active.updated_at = Set(now);
        Ok(Some(try_friendship(active.update(&self.database).await?)?))
    }

    async fn friendships_for_user(
        &self,
        user_id: &Uuid,
        status: FriendshipStatus,
    ) -> anyhow::Result<Vec<Friendship>> {
        rows_to_friendships(
            friendships::Entity::find()
                .filter(friendships::Column::Status.eq(status.as_str()))
                .filter(
                    Condition::any()
                        .add(friendships::Column::RequesterUserId.eq(*user_id))
                        .add(friendships::Column::RecipientUserId.eq(*user_id)),
                )
                .order_by_desc(friendships::Column::UpdatedAt)
                .all(&self.database)
                .await?,
        )
    }

    async fn incoming_requests(&self, user_id: &Uuid) -> anyhow::Result<Vec<Friendship>> {
        request_rows(
            &self.database,
            friendships::Column::RecipientUserId,
            user_id,
        )
        .await
    }

    async fn outgoing_requests(&self, user_id: &Uuid) -> anyhow::Result<Vec<Friendship>> {
        request_rows(
            &self.database,
            friendships::Column::RequesterUserId,
            user_id,
        )
        .await
    }

    async fn conversation_by_id(
        &self,
        conversation_id: &Uuid,
    ) -> anyhow::Result<Option<DmConversation>> {
        Ok(dm_conversations::Entity::find_by_id(*conversation_id)
            .one(&self.database)
            .await?
            .map(Into::into))
    }

    async fn conversations_for_user(&self, user_id: &Uuid) -> anyhow::Result<Vec<DmConversation>> {
        Ok(dm_conversations::Entity::find()
            .filter(
                Condition::any()
                    .add(dm_conversations::Column::UserLowId.eq(*user_id))
                    .add(dm_conversations::Column::UserHighId.eq(*user_id)),
            )
            .order_by_desc(dm_conversations::Column::UpdatedAt)
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn get_or_create_conversation(
        &self,
        left_user_id: &Uuid,
        right_user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<DmConversation> {
        let (user_low_id, user_high_id) = ordered_pair(*left_user_id, *right_user_id);
        if let Some(row) = dm_conversations::Entity::find()
            .filter(dm_conversations::Column::UserLowId.eq(user_low_id))
            .filter(dm_conversations::Column::UserHighId.eq(user_high_id))
            .one(&self.database)
            .await?
        {
            return Ok(row.into());
        }

        Ok(dm_conversations::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_low_id: Set(user_low_id),
            user_high_id: Set(user_high_id),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.database)
        .await?
        .into())
    }

    async fn dm_message_page(
        &self,
        conversation_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<DmMessagePage> {
        let before_message = match before_message_id {
            Some(message_id) => Some(
                dm_messages::Entity::find()
                    .filter(dm_messages::Column::ConversationId.eq(*conversation_id))
                    .filter(dm_messages::Column::Id.eq(*message_id))
                    .one(&self.database)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("dm message history cursor was not found"))?,
            ),
            None => None,
        };
        let mut filter = Condition::all()
            .add(dm_messages::Column::ConversationId.eq(*conversation_id))
            .add(dm_messages::Column::DeletedAt.is_null());
        if let Some(message) = before_message {
            filter = filter.add(
                Condition::any()
                    .add(dm_messages::Column::CreatedAt.lt(message.created_at))
                    .add(
                        Condition::all()
                            .add(dm_messages::Column::CreatedAt.eq(message.created_at))
                            .add(dm_messages::Column::Id.lt(message.id)),
                    ),
            );
        }

        let mut messages = dm_messages::Entity::find()
            .filter(filter)
            .order_by_desc(dm_messages::Column::CreatedAt)
            .order_by_desc(dm_messages::Column::Id)
            .limit(DM_HISTORY_LIMIT + 1)
            .all(&self.database)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let has_more = messages.len() > usize::try_from(DM_HISTORY_LIMIT).unwrap_or(50);
        if has_more {
            messages.truncate(usize::try_from(DM_HISTORY_LIMIT).unwrap_or(50));
        }
        messages.sort_by_key(|message: &DmMessage| (message.created_at, message.id));
        Ok(DmMessagePage { messages, has_more })
    }

    async fn insert_dm_message(&self, message: DmMessage) -> anyhow::Result<DmMessage> {
        let inserted = dm_messages::ActiveModel {
            id: Set(message.id),
            conversation_id: Set(message.conversation_id),
            sender_user_id: Set(message.sender_user_id),
            body: Set(message.body),
            created_at: Set(message.created_at),
            updated_at: Set(message.updated_at),
            deleted_at: Set(message.deleted_at),
        }
        .insert(&self.database)
        .await?;

        if let Some(row) = dm_conversations::Entity::find_by_id(inserted.conversation_id)
            .one(&self.database)
            .await?
        {
            let mut active = row.into_active_model();
            active.updated_at = Set(inserted.created_at);
            active.update(&self.database).await?;
        }

        Ok(inserted.into())
    }
}

async fn request_rows(
    database: &DatabaseConnection,
    user_column: friendships::Column,
    user_id: &Uuid,
) -> anyhow::Result<Vec<Friendship>> {
    rows_to_friendships(
        friendships::Entity::find()
            .filter(friendships::Column::Status.eq(FriendshipStatus::Pending.as_str()))
            .filter(user_column.eq(*user_id))
            .order_by_desc(friendships::Column::CreatedAt)
            .all(database)
            .await?,
    )
}

fn rows_to_friendships(rows: Vec<friendships::Model>) -> anyhow::Result<Vec<Friendship>> {
    rows.into_iter().map(try_friendship).collect()
}

fn try_friendship(row: friendships::Model) -> anyhow::Result<Friendship> {
    let status = FriendshipStatus::from_str(&row.status)
        .ok_or_else(|| anyhow::anyhow!("unknown friendship status {}", row.status))?;
    Ok(Friendship {
        id: row.id,
        requester_user_id: row.requester_user_id,
        recipient_user_id: row.recipient_user_id,
        user_low_id: row.user_low_id,
        user_high_id: row.user_high_id,
        status,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

impl From<dm_conversations::Model> for DmConversation {
    fn from(row: dm_conversations::Model) -> Self {
        Self {
            id: row.id,
            user_low_id: row.user_low_id,
            user_high_id: row.user_high_id,
            updated_at: row.updated_at,
        }
    }
}

impl From<dm_messages::Model> for DmMessage {
    fn from(row: dm_messages::Model) -> Self {
        Self {
            id: row.id,
            conversation_id: row.conversation_id,
            sender_user_id: row.sender_user_id,
            body: row.body,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        }
    }
}
