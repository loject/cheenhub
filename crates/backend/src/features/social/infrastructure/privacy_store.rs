//! Контракт постоянного хранения индивидуальной приватности.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::social::domain::{PrivacySettings, UserBlock};

/// Независимые настройки и направленные блокировки пользователей.
#[async_trait]
pub(crate) trait PrivacyStore: Send + Sync {
    /// Возвращает настройки или значения по умолчанию.
    async fn privacy_settings(&self, user_id: &Uuid) -> anyhow::Result<PrivacySettings>;
    /// Сохраняет настройки и момент последнего ограничения ЛС.
    async fn save_privacy_settings(
        &self,
        user_id: &Uuid,
        settings: PrivacySettings,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    /// Возвращает блокировки пары в обоих направлениях, включая снятые.
    async fn user_blocks_between(
        &self,
        left: &Uuid,
        right: &Uuid,
    ) -> anyhow::Result<Vec<UserBlock>>;
    /// Возвращает активные блокировки владельца.
    async fn active_user_blocks(&self, user_id: &Uuid) -> anyhow::Result<Vec<UserBlock>>;
    /// Блокирует пользователя и отменяет дружбу/ожидающую заявку.
    async fn block_user(
        &self,
        blocker: &Uuid,
        blocked: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<UserBlock>;
    /// Снимает блокировку с сохранением истории её отзыва.
    async fn unblock_user(
        &self,
        blocker: &Uuid,
        blocked: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool>;
    /// Проверяет наличие сообщения автора строго после указанного времени.
    /// Мягко удалённые сообщения тоже учитываются как предоставленное разрешение.
    async fn has_dm_message_after(
        &self,
        conversation_id: &Uuid,
        author_id: &Uuid,
        after: DateTime<Utc>,
    ) -> anyhow::Result<bool>;
}
