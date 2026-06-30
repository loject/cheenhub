//! Доменные модели дружбы и личных сообщений.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Статус записи дружбы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FriendshipStatus {
    /// Заявка ожидает решения.
    Pending,
    /// Пользователи являются друзьями.
    Accepted,
    /// Заявка отклонена.
    Declined,
    /// Заявка отменена отправителем или дружба удалена.
    Cancelled,
}

impl FriendshipStatus {
    /// Возвращает строковое значение для хранения.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Cancelled => "cancelled",
        }
    }

    /// Читает статус из строки хранилища.
    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "accepted" => Some(Self::Accepted),
            "declined" => Some(Self::Declined),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Связь дружбы или заявка между двумя пользователями.
#[derive(Debug, Clone)]
pub(crate) struct Friendship {
    /// Стабильный идентификатор связи.
    pub(crate) id: Uuid,
    /// Пользователь, создавший текущую заявку.
    pub(crate) requester_user_id: Uuid,
    /// Пользователь, получивший текущую заявку.
    pub(crate) recipient_user_id: Uuid,
    /// Меньший UUID пары пользователей.
    pub(crate) user_low_id: Uuid,
    /// Больший UUID пары пользователей.
    pub(crate) user_high_id: Uuid,
    /// Текущий статус связи.
    pub(crate) status: FriendshipStatus,
    /// Время создания связи.
    pub(crate) created_at: DateTime<Utc>,
    /// Время последнего обновления связи.
    pub(crate) updated_at: DateTime<Utc>,
}

/// Диалог личных сообщений.
#[derive(Debug, Clone)]
pub(crate) struct DmConversation {
    /// Стабильный идентификатор диалога.
    pub(crate) id: Uuid,
    /// Меньший UUID пары пользователей.
    pub(crate) user_low_id: Uuid,
    /// Больший UUID пары пользователей.
    pub(crate) user_high_id: Uuid,
    /// Время последнего обновления диалога.
    pub(crate) updated_at: DateTime<Utc>,
}

/// Сообщение личного диалога.
#[derive(Debug, Clone)]
pub(crate) struct DmMessage {
    /// Стабильный идентификатор сообщения.
    pub(crate) id: Uuid,
    /// Диалог, которому принадлежит сообщение.
    pub(crate) conversation_id: Uuid,
    /// Монотонный порядковый номер сообщения внутри диалога.
    pub(crate) seq: i64,
    /// Пользователь, отправивший сообщение.
    pub(crate) sender_user_id: Uuid,
    /// Текст сообщения.
    pub(crate) body: String,
    /// Время создания.
    pub(crate) created_at: DateTime<Utc>,
    /// Время последнего обновления.
    pub(crate) updated_at: DateTime<Utc>,
    /// Время мягкого удаления.
    pub(crate) deleted_at: Option<DateTime<Utc>>,
}

/// Текущее состояние прочтения участника личного диалога.
#[derive(Debug, Clone)]
pub(crate) struct ConversationMemberState {
    /// Диалог, к которому относится состояние.
    pub(crate) conversation_id: Uuid,
    /// Пользователь-участник диалога.
    pub(crate) user_id: Uuid,
    /// Последнее прочитанное сообщение.
    pub(crate) last_read_message_id: Option<Uuid>,
    /// Последний прочитанный порядковый номер.
    pub(crate) last_read_seq: i64,
    /// Серверное время последнего подтверждения прочтения.
    pub(crate) last_read_at: Option<DateTime<Utc>>,
    /// Количество непрочитанных входящих сообщений.
    pub(crate) unread_count: i64,
    /// Время последнего обновления состояния.
    pub(crate) updated_at: DateTime<Utc>,
}

/// Исторический checkpoint прочтения личного диалога.
#[derive(Debug, Clone)]
pub(crate) struct ConversationReadCheckpoint {
    /// Стабильный идентификатор checkpoint.
    pub(crate) id: Uuid,
    /// Диалог, к которому относится checkpoint.
    pub(crate) conversation_id: Uuid,
    /// Пользователь, который прочитал сообщения.
    pub(crate) user_id: Uuid,
    /// Последнее прочитанное сообщение.
    pub(crate) last_read_message_id: Uuid,
    /// Последний прочитанный порядковый номер.
    pub(crate) last_read_seq: i64,
    /// Серверное время подтверждения прочтения.
    pub(crate) read_at: DateTime<Utc>,
    /// Время создания записи.
    pub(crate) created_at: DateTime<Utc>,
}

/// Итог обновления read-state личного диалога.
#[derive(Debug, Clone)]
pub(crate) struct ConversationReadUpdate {
    /// Текущее состояние участника после операции.
    pub(crate) state: ConversationMemberState,
    /// Новый checkpoint, если операция продвинула read-state.
    pub(crate) checkpoint: Option<ConversationReadCheckpoint>,
}

/// Возвращает UUID пары в стабильном порядке.
pub(crate) fn ordered_pair(left: Uuid, right: Uuid) -> (Uuid, Uuid) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}
