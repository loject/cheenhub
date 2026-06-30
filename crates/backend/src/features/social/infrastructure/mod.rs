//! Инфраструктурный слой друзей и личных сообщений.

mod entities;
mod in_memory;
mod postgres;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::social::domain::{DmConversation, DmMessage, Friendship, FriendshipStatus};

pub(crate) use in_memory::InMemorySocialStore;
pub(crate) use postgres::PostgresSocialStore;

pub(crate) const DM_HISTORY_LIMIT: u64 = 50;

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

    /// Вставляет личное сообщение и обновляет время диалога.
    async fn insert_dm_message(&self, message: DmMessage) -> anyhow::Result<DmMessage>;
}
