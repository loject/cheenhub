//! Преобразования строк Postgres social-хранилища в доменные структуры.

use crate::features::social::domain::{
    ConversationMemberState, ConversationReadCheckpoint, DmConversation, DmMessage,
};
use crate::features::social::infrastructure::entities::{
    conversation_member_states, conversation_read_checkpoints, dm_conversations, dm_messages,
};
use crate::features::social::infrastructure::normalize_unread_count;

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
            seq: row.seq,
            sender_user_id: row.sender_user_id,
            body: row.body,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
        }
    }
}

impl From<conversation_member_states::Model> for ConversationMemberState {
    fn from(row: conversation_member_states::Model) -> Self {
        Self {
            conversation_id: row.conversation_id,
            user_id: row.user_id,
            last_read_message_id: row.last_read_message_id,
            last_read_seq: row.last_read_seq,
            last_read_at: row.last_read_at,
            unread_count: normalize_unread_count(row.unread_count),
            updated_at: row.updated_at,
        }
    }
}

impl From<conversation_read_checkpoints::Model> for ConversationReadCheckpoint {
    fn from(row: conversation_read_checkpoints::Model) -> Self {
        Self {
            id: row.id,
            conversation_id: row.conversation_id,
            user_id: row.user_id,
            last_read_message_id: row.last_read_message_id,
            last_read_seq: row.last_read_seq,
            read_at: row.read_at,
            created_at: row.created_at,
        }
    }
}
