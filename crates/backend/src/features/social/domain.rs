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

/// Данные друга, необходимые для сортировки и preview списка.
#[derive(Debug, Clone)]
pub(crate) struct FriendListEntry {
    /// Принятая связь дружбы.
    pub(crate) friendship: Friendship,
    /// Идентификатор пользователя-друга.
    pub(crate) friend_user_id: Uuid,
    /// Количество непрочитанных сообщений от друга.
    pub(crate) unread_count: i64,
    /// Последнее неудалённое сообщение диалога.
    pub(crate) last_message: Option<DmMessage>,
}

/// Позиция keyset-пагинации в отсортированном списке друзей.
#[derive(Debug, Clone)]
pub(crate) struct FriendListCursor {
    /// Время последнего сообщения или `None` для части списка без сообщений.
    pub(crate) last_message_created_at: Option<DateTime<Utc>>,
    /// Идентификатор друга для стабильного разрешения совпавшего времени.
    pub(crate) friend_user_id: Uuid,
}

/// Позиция в общем списке друзей и личных собеседников.
#[derive(Debug, Clone)]
pub(crate) struct SocialListCursor {
    /// Время последнего неудалённого сообщения, если оно существует.
    pub(crate) last_message_created_at: Option<DateTime<Utc>>,
    /// Нормализованный никнейм для разрешения равных временных отметок.
    pub(crate) nickname_key: String,
    /// UUID собеседника для окончательного разрешения совпадений.
    pub(crate) user_id: Uuid,
}

/// Строка социальной выборки после объединения друзей и собеседников.
#[derive(Debug, Clone)]
pub(crate) struct SocialListEntry {
    /// Идентификатор собеседника.
    pub(crate) user_id: Uuid,
    /// Нормализованный никнейм, по которому выполнялись поиск и сортировка.
    pub(crate) nickname_key: String,
    /// Существующий диалог, отсутствующий у друзей без переписки.
    pub(crate) conversation: Option<DmConversation>,
    /// Состояние прочтения диалога текущим пользователем.
    pub(crate) read_state: Option<ConversationMemberState>,
    /// Последнее неудалённое сообщение, определяющее порядок.
    pub(crate) last_message: Option<DmMessage>,
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
    /// Идентификатор прикреплённого изображения.
    pub(crate) image_id: Option<Uuid>,
    /// Время создания.
    pub(crate) created_at: DateTime<Utc>,
    /// Время последнего обновления.
    pub(crate) updated_at: DateTime<Utc>,
    /// Время мягкого удаления.
    pub(crate) deleted_at: Option<DateTime<Utc>>,
}

/// Число личных сообщений, отправленных в одну минуту окна статистики.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DmMessagesPerMinute {
    /// Начало минуты в UTC, выровненное по границе минуты.
    pub(crate) minute: DateTime<Utc>,
    /// Число сообщений, созданных за эту минуту.
    pub(crate) messages: u64,
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

/// Правило допуска входящих личных звонков и приглашений.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IncomingContactRule {
    /// Контакт может начать любой незаблокированный пользователь.
    Everyone,
    /// Контакт могут начать только друзья.
    Friends,
    /// Входящие контакты запрещены всем.
    Nobody,
}

impl IncomingContactRule {
    /// Значение для записи в PostgreSQL.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Everyone => "everyone",
            Self::Friends => "friends",
            Self::Nobody => "nobody",
        }
    }

    /// Разбирает сохранённое значение без неявного разрешения неизвестной политики.
    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "everyone" => Some(Self::Everyone),
            "friends" => Some(Self::Friends),
            "nobody" => Some(Self::Nobody),
            _ => None,
        }
    }
}

/// Настройки входящих контактов с контрольной датой ограничений ЛС.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PrivacySettings {
    /// Требуется ли дружба или сообщение владельца для входящих ЛС.
    pub(crate) direct_messages_friends_only: bool,
    /// Кто может звонить владельцу.
    pub(crate) direct_calls: IncomingContactRule,
    /// Кто может приглашать владельца в голосовые комнаты.
    pub(crate) voice_room_invites: IncomingContactRule,
    /// Время последнего переключения ЛС на «Только друзья».
    pub(crate) direct_messages_restricted_since: Option<DateTime<Utc>>,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            direct_messages_friends_only: false,
            direct_calls: IncomingContactRule::Friends,
            voice_room_invites: IncomingContactRule::Friends,
            direct_messages_restricted_since: None,
        }
    }
}

/// Последняя блокировка направленной пары пользователей.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserBlock {
    /// Стабильный идентификатор записи, сохраняемый после разблокировки.
    pub(crate) id: Uuid,
    /// Пользователь, установивший блокировку.
    pub(crate) blocker_user_id: Uuid,
    /// Пользователь, на которого действует блокировка.
    pub(crate) blocked_user_id: Uuid,
    /// Время последней установки блокировки.
    pub(crate) blocked_at: DateTime<Utc>,
    /// Время снятия последней блокировки, если она снята.
    pub(crate) unblocked_at: Option<DateTime<Utc>>,
}
