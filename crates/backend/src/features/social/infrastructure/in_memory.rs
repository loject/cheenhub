//! Простое in-memory-хранилище друзей и личных сообщений.

use std::sync::Mutex;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::social::domain::{
    ConversationMemberState, ConversationReadCheckpoint, ConversationReadUpdate, DmConversation,
    DmMessage, Friendship, FriendshipStatus, ordered_pair,
};
use crate::features::social::infrastructure::{
    DM_HISTORY_LIMIT, DmMessagePage, SocialStore, normalize_unread_count, unread_count_after_read,
};

/// In-memory-хранилище социальных данных для локального режима и тестов.
#[derive(Default)]
pub(crate) struct InMemorySocialStore {
    friendships: Mutex<Vec<Friendship>>,
    conversations: Mutex<Vec<DmConversation>>,
    messages: Mutex<Vec<DmMessage>>,
    member_states: Mutex<Vec<ConversationMemberState>>,
    read_checkpoints: Mutex<Vec<ConversationReadCheckpoint>>,
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
        now: DateTime<Utc>,
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
        now: DateTime<Utc>,
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
        now: DateTime<Utc>,
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
        drop(conversations);
        self.ensure_member_state(&conversation.id, &user_low_id, now)?;
        self.ensure_member_state(&conversation.id, &user_high_id, now)?;
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

    async fn dm_message_by_id(
        &self,
        conversation_id: &Uuid,
        message_id: &Uuid,
    ) -> anyhow::Result<Option<DmMessage>> {
        Ok(self
            .messages
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .find(|row| row.conversation_id == *conversation_id && row.id == *message_id)
            .cloned())
    }

    async fn conversation_member_state(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ConversationMemberState>> {
        Ok(self
            .member_states
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .find(|row| row.conversation_id == *conversation_id && row.user_id == *user_id)
            .cloned())
    }

    async fn total_unread_count(&self, user_id: &Uuid) -> anyhow::Result<i64> {
        Ok(self
            .member_states
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| row.user_id == *user_id)
            .map(|row| normalize_unread_count(row.unread_count))
            .sum())
    }

    async fn mark_conversation_read(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        last_read_message_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<ConversationReadUpdate> {
        let message = self
            .dm_message_by_id(conversation_id, last_read_message_id)
            .await?
            .ok_or_else(|| anyhow!("dm read message was not found in conversation"))?;
        let mut member_states = self.member_states.lock().map_err(|_| poisoned())?;
        let state_index = member_states
            .iter()
            .position(|row| row.conversation_id == *conversation_id && row.user_id == *user_id)
            .unwrap_or_else(|| {
                member_states.push(default_member_state(conversation_id, user_id, now));
                member_states.len() - 1
            });
        let current = member_states[state_index].clone();
        if message.seq <= current.last_read_seq {
            return Ok(ConversationReadUpdate {
                state: current,
                checkpoint: None,
            });
        }

        let incoming_read = self
            .messages
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| {
                row.conversation_id == *conversation_id
                    && row.deleted_at.is_none()
                    && row.sender_user_id != *user_id
                    && row.seq > current.last_read_seq
                    && row.seq <= message.seq
            })
            .count() as i64;
        let checkpoint = ConversationReadCheckpoint {
            id: Uuid::new_v4(),
            conversation_id: *conversation_id,
            user_id: *user_id,
            last_read_message_id: *last_read_message_id,
            last_read_seq: message.seq,
            read_at: now,
            created_at: now,
        };
        let next_state = ConversationMemberState {
            conversation_id: *conversation_id,
            user_id: *user_id,
            last_read_message_id: Some(*last_read_message_id),
            last_read_seq: message.seq,
            last_read_at: Some(now),
            unread_count: unread_count_after_read(current.unread_count, incoming_read),
            updated_at: now,
        };
        member_states[state_index] = next_state.clone();
        self.read_checkpoints
            .lock()
            .map_err(|_| poisoned())?
            .push(checkpoint.clone());
        Ok(ConversationReadUpdate {
            state: next_state,
            checkpoint: Some(checkpoint),
        })
    }

    #[cfg(test)]
    async fn message_read_at(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        message_seq: i64,
    ) -> anyhow::Result<Option<DateTime<Utc>>> {
        let mut checkpoints = self
            .read_checkpoints
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| {
                row.conversation_id == *conversation_id
                    && row.user_id == *user_id
                    && row.last_read_seq >= message_seq
            })
            .cloned()
            .collect::<Vec<_>>();
        checkpoints.sort_by_key(|row| (row.last_read_seq, row.read_at));
        Ok(checkpoints.first().map(|row| row.read_at))
    }

    async fn insert_dm_message(&self, message: DmMessage) -> anyhow::Result<DmMessage> {
        let next_seq = self
            .messages
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|row| row.conversation_id == message.conversation_id)
            .map(|row| row.seq)
            .max()
            .unwrap_or(0)
            + 1;
        let message = DmMessage {
            seq: next_seq,
            ..message
        };
        {
            let mut conversations = self.conversations.lock().map_err(|_| poisoned())?;
            if let Some(conversation) = conversations
                .iter_mut()
                .find(|row| row.id == message.conversation_id)
            {
                conversation.updated_at = message.created_at;
                let recipient_user_id = if conversation.user_low_id == message.sender_user_id {
                    conversation.user_high_id
                } else {
                    conversation.user_low_id
                };
                self.increment_unread(
                    &message.conversation_id,
                    &recipient_user_id,
                    message.created_at,
                )?;
            }
        }
        self.messages
            .lock()
            .map_err(|_| poisoned())?
            .push(message.clone());
        Ok(message)
    }
}

impl InMemorySocialStore {
    fn ensure_member_state(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut member_states = self.member_states.lock().map_err(|_| poisoned())?;
        if !member_states
            .iter()
            .any(|row| row.conversation_id == *conversation_id && row.user_id == *user_id)
        {
            member_states.push(default_member_state(conversation_id, user_id, now));
        }
        Ok(())
    }

    fn increment_unread(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut member_states = self.member_states.lock().map_err(|_| poisoned())?;
        if let Some(row) = member_states
            .iter_mut()
            .find(|row| row.conversation_id == *conversation_id && row.user_id == *user_id)
        {
            row.unread_count = normalize_unread_count(row.unread_count) + 1;
            row.updated_at = now;
        } else {
            let mut state = default_member_state(conversation_id, user_id, now);
            state.unread_count = 1;
            member_states.push(state);
        }
        Ok(())
    }
}

fn default_member_state(
    conversation_id: &Uuid,
    user_id: &Uuid,
    now: DateTime<Utc>,
) -> ConversationMemberState {
    ConversationMemberState {
        conversation_id: *conversation_id,
        user_id: *user_id,
        last_read_message_id: None,
        last_read_seq: 0,
        last_read_at: None,
        unread_count: 0,
        updated_at: now,
    }
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory social store lock poisoned")
}
