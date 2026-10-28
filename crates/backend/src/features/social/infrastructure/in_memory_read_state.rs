//! Общие операции и начальное состояние прочтения в локальном social-хранилище.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::social::domain::ConversationMemberState;

use super::in_memory::{InMemorySocialStore, poisoned};
use super::normalize_unread_count;

impl InMemorySocialStore {
    /// Создаёт пустое состояние прочтения, если участник ещё не зарегистрирован.
    pub(super) fn ensure_member_state(
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

    /// Увеличивает счётчик непрочитанных входящих сообщений получателя.
    pub(super) fn increment_unread(
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

/// Создаёт начальное состояние участника без прочитанных сообщений.
pub(super) fn default_member_state(
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
