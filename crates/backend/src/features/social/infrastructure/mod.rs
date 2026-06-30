//! Инфраструктурный слой друзей и личных сообщений.

mod entities;
mod in_memory;
mod postgres;
mod postgres_conversions;
mod postgres_read_state;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::social::domain::{
    ConversationMemberState, ConversationReadUpdate, DmConversation, DmMessage, Friendship,
    FriendshipStatus,
};

pub(crate) use in_memory::InMemorySocialStore;
pub(crate) use postgres::PostgresSocialStore;

pub(crate) const DM_HISTORY_LIMIT: u64 = 50;

/// Приводит счетчик непрочитанных к допустимому диапазону.
pub(crate) fn normalize_unread_count(unread_count: i64) -> i64 {
    unread_count.max(0)
}

/// Возвращает счетчик после отметки входящих сообщений прочитанными.
pub(crate) fn unread_count_after_read(current_unread_count: i64, incoming_read: i64) -> i64 {
    (normalize_unread_count(current_unread_count) - incoming_read.max(0)).max(0)
}

/// Страница сообщений личного диалога.
pub(crate) struct DmMessagePage {
    /// Сообщения в порядке от старых к новым.
    pub(crate) messages: Vec<DmMessage>,
    /// Есть ли более старые сообщения.
    pub(crate) has_more: bool,
}

/// Граница хранения социальных связей и личных сообщений.
#[async_trait]
pub(crate) trait SocialStore: Send + Sync {
    /// Находит запись дружбы для пары пользователей.
    async fn friendship_between(
        &self,
        left_user_id: &Uuid,
        right_user_id: &Uuid,
    ) -> anyhow::Result<Option<Friendship>>;

    /// Находит запись дружбы по идентификатору.
    async fn friendship_by_id(&self, friendship_id: &Uuid) -> anyhow::Result<Option<Friendship>>;

    /// Создает или переоткрывает заявку в друзья для пары пользователей.
    async fn upsert_friend_request(
        &self,
        requester_user_id: &Uuid,
        recipient_user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Friendship>;

    /// Меняет статус записи дружбы.
    async fn update_friendship_status(
        &self,
        friendship_id: &Uuid,
        status: FriendshipStatus,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<Friendship>>;

    /// Возвращает дружбы пользователя с указанным статусом.
    async fn friendships_for_user(
        &self,
        user_id: &Uuid,
        status: FriendshipStatus,
    ) -> anyhow::Result<Vec<Friendship>>;

    /// Возвращает входящие заявки пользователя.
    async fn incoming_requests(&self, user_id: &Uuid) -> anyhow::Result<Vec<Friendship>>;

    /// Возвращает исходящие заявки пользователя.
    async fn outgoing_requests(&self, user_id: &Uuid) -> anyhow::Result<Vec<Friendship>>;

    /// Находит личный диалог по идентификатору.
    async fn conversation_by_id(
        &self,
        conversation_id: &Uuid,
    ) -> anyhow::Result<Option<DmConversation>>;

    /// Возвращает личные диалоги пользователя.
    async fn conversations_for_user(&self, user_id: &Uuid) -> anyhow::Result<Vec<DmConversation>>;

    /// Создает диалог пары или возвращает существующий.
    async fn get_or_create_conversation(
        &self,
        left_user_id: &Uuid,
        right_user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<DmConversation>;

    /// Возвращает страницу сообщений диалога.
    async fn dm_message_page(
        &self,
        conversation_id: &Uuid,
        before_message_id: Option<&Uuid>,
    ) -> anyhow::Result<DmMessagePage>;

    /// Возвращает одно сообщение личного диалога.
    async fn dm_message_by_id(
        &self,
        conversation_id: &Uuid,
        message_id: &Uuid,
    ) -> anyhow::Result<Option<DmMessage>>;

    /// Возвращает read-state участника диалога.
    async fn conversation_member_state(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<Option<ConversationMemberState>>;

    /// Возвращает суммарное количество непрочитанных личных сообщений пользователя.
    async fn total_unread_count(&self, user_id: &Uuid) -> anyhow::Result<i64>;

    /// Продвигает read-state участника до указанного сообщения.
    async fn mark_conversation_read(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        last_read_message_id: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<ConversationReadUpdate>;

    /// Возвращает время прочтения сообщения по первому подходящему checkpoint.
    #[cfg(test)]
    async fn message_read_at(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        message_seq: i64,
    ) -> anyhow::Result<Option<DateTime<Utc>>>;

    /// Вставляет личное сообщение и обновляет время диалога.
    async fn insert_dm_message(&self, message: DmMessage) -> anyhow::Result<DmMessage>;
}

#[cfg(test)]
mod tests {
    use super::{normalize_unread_count, unread_count_after_read};

    #[test]
    fn unread_count_after_read_never_goes_below_zero() {
        assert_eq!(unread_count_after_read(0, 77), 0);
        assert_eq!(unread_count_after_read(-77, 1), 0);
        assert_eq!(unread_count_after_read(5, 2), 3);
    }

    #[test]
    fn normalize_unread_count_repairs_legacy_negative_values() {
        assert_eq!(normalize_unread_count(-76), 0);
        assert_eq!(normalize_unread_count(3), 3);
    }
}
