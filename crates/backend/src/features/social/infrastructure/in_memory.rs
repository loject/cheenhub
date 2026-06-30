//! Простое in-memory-хранилище друзей и личных сообщений.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use uuid::Uuid;

use crate::features::social::domain::{
    DmConversation, DmMessage, Friendship, FriendshipStatus, ordered_pair,
};
use crate::features::social::infrastructure::{DM_HISTORY_LIMIT, DmMessagePage, SocialStore};

/// In-memory-хранилище социальных данных для локального режима и тестов.
#[derive(Default)]
pub(crate) struct InMemorySocialStore {
    friendships: Mutex<Vec<Friendship>>,
    conversations: Mutex<Vec<DmConversation>>,
    messages: Mutex<Vec<DmMessage>>,
}

#[async_trait]
impl SocialStore for InMemorySocialStore {
    async fn friendship_between(
        &self,
        left_user_id: &Uuid,
        right_user_id: &Uuid,
    ) -> anyhow::Result<Option<Friendship>> {
        let (user_low_id, user_high_id) = ordered_pair(*left_user_id, *right_user_id);
        Ok(self
            .friendships
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .find(|row| row.user_low_id == user_low_id && row.user_high_id == user_high_id)
            .cloned())
    }

    async fn friendship_by_id(&self, friendship_id: &Uuid) -> anyhow::Result<Option<Friendship>> {
        Ok(self
            .friendships
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .find(|row| row.id == *friendship_id)
            .cloned())
    }

    async fn upsert_friend_request(
        &self,
        requester_user_id: &Uuid,
        recipient_user_id: &Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Friendship> {
        let (user_low_id, user_high_id) = ordered_pair(*requester_user_id, *recipient_user_id);
        let mut friendships = self.friendships.lock().map_err(|_| poisoned())?;
        if let Some(row) = friendships
            .iter_mut()
            .find(|row| row.user_low_id == user_low_id && row.user_high_id == user_high_id)
        {
            row.requester_user_id = *requester_user_id;
            row.recipient_user_id = *recipient_user_id;
            row.status = FriendshipStatus::Pending;
            row.updated_at = now;
            return Ok(row.clone());
        }

        let friendship = Friendship {
            id: Uuid::new_v4(),
            requester_user_id: *requester_user_id,
            recipient_user_id: *recipient_user_id,
            user_low_id,
            user_high_id,
            status: FriendshipStatus::Pending,
            created_at: now,
            updated_at: now,
        };
        friendships.push(friendship.clone());
        Ok(friendship)
    }

    async fn update_friendship_status(
        &self,
        friendship_id: &Uuid,
        status: FriendshipStatus,
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Option<Friendship>> {
        let mut friendships = self.friendships.lock().map_err(|_| poisoned())?;
        let Some(row) = friendships.iter_mut().find(|row| row.id == *friendship_id) else {
            return Ok(None);
        };
        row.status = status;
        row.updated_at = now;
        Ok(Some(row.clone()))
    }

    async fn friendships_for_user(
        &self,
        user_id: &Uuid,
        status: FriendshipStatus,
    ) -> anyhow::Result<Vec<Friendship>> {
        Ok(self
            .friendships
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| {
                row.status == status
                    && (row.requester_user_id == *user_id || row.recipient_user_id == *user_id)
            })
            .cloned()
            .collect())
    }

    async fn incoming_requests(&self, user_id: &Uuid) -> anyhow::Result<Vec<Friendship>> {
        Ok(self
            .friendships
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| {
                row.status == FriendshipStatus::Pending && row.recipient_user_id == *user_id
            })
            .cloned()
            .collect())
    }

    async fn outgoing_requests(&self, user_id: &Uuid) -> anyhow::Result<Vec<Friendship>> {
        Ok(self
            .friendships
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| {
                row.status == FriendshipStatus::Pending && row.requester_user_id == *user_id
            })
            .cloned()
            .collect())
    }

    async fn conversation_by_id(
        &self,
        conversation_id: &Uuid,
    ) -> anyhow::Result<Option<DmConversation>> {
        Ok(self
            .conversations
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .find(|row| row.id == *conversation_id)
            .cloned())
    }

    async fn conversations_for_user(&self, user_id: &Uuid) -> anyhow::Result<Vec<DmConversation>> {
        let mut conversations = self
            .conversations
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| row.user_low_id == *user_id || row.user_high_id == *user_id)
            .cloned()
            .collect::<Vec<_>>();
        conversations.sort_by_key(|row| row.updated_at);
        conversations.reverse();
        Ok(conversations)
    }

    async fn get_or_create_conversation(
        &self,
        left_user_id: &Uuid,
        right_user_id: &Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<DmConversation> {
        let (user_low_id, user_high_id) = ordered_pair(*left_user_id, *right_user_id);
        let mut conversations = self.conversations.lock().map_err(|_| poisoned())?;
        if let Some(row) = conversations
            .iter()
            .find(|row| row.user_low_id == user_low_id && row.user_high_id == user_high_id)
        {
            return Ok(row.clone());
        }

        let conversation = DmConversation {
            id: Uuid::new_v4(),
            user_low_id,
            user_high_id,
            updated_at: now,
        };
        conversations.push(conversation.clone());
        Ok(conversation)
    }

    async fn dm_message_page(
        &self,
        conversation_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<DmMessagePage> {
        let mut messages = self
            .messages
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| row.conversation_id == *conversation_id && row.deleted_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        messages.sort_by_key(|row| (row.created_at, row.id));
        if let Some(before_message_id) = before_message_id {
            let Some(cursor_index) = messages.iter().position(|row| row.id == *before_message_id)
            else {
                return Err(anyhow!("dm message history cursor was not found"));
            };
            messages.truncate(cursor_index);
        }
        let start = messages
            .len()
            .saturating_sub(usize::try_from(DM_HISTORY_LIMIT).unwrap_or(50));
        let has_more = start > 0;
        Ok(DmMessagePage {
            messages: messages.split_off(start),
            has_more,
        })
    }

    async fn insert_dm_message(&self, message: DmMessage) -> anyhow::Result<DmMessage> {
        {
            let mut conversations = self.conversations.lock().map_err(|_| poisoned())?;
            if let Some(conversation) = conversations
                .iter_mut()
                .find(|row| row.id == message.conversation_id)
            {
                conversation.updated_at = message.created_at;
            }
        }
        self.messages
            .lock()
            .map_err(|_| poisoned())?
            .push(message.clone());
        Ok(message)
    }
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory social store lock poisoned")
}
