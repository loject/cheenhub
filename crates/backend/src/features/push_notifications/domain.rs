//! Модель данных системных push-уведомлений.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const IMAGE_MESSAGE_PREVIEW: &str = "Изображение";

/// Возвращает непустой пользовательский preview сообщения.
pub(crate) fn direct_message_preview(body: &str, has_image: bool) -> String {
    if body.trim().is_empty() && has_image {
        IMAGE_MESSAGE_PREVIEW.to_owned()
    } else {
        body.to_owned()
    }
}

/// Содержимое push-уведомления о новом личном сообщении.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DirectMessagePush {
    /// Версия схемы data payload.
    pub(crate) schema_version: String,
    /// Машиночитаемый вид события.
    pub(crate) kind: String,
    /// Идентификатор сообщения для дедупликации.
    pub(crate) message_id: String,
    /// Идентификатор личного диалога.
    pub(crate) conversation_id: String,
    /// Порядковый номер сообщения внутри диалога.
    pub(crate) message_seq: String,
    /// Идентификатор отправителя.
    pub(crate) sender_user_id: String,
    /// Отображаемое имя отправителя.
    pub(crate) sender_nickname: String,
    /// Безопасно ограниченный текст для системного уведомления.
    pub(crate) body_preview: String,
    /// RFC 3339 время создания сообщения.
    pub(crate) created_at: String,
}

impl DirectMessagePush {
    /// Собирает payload, совпадающий с контрактом Android-обработчика.
    pub(crate) fn new(
        message_id: Uuid,
        conversation_id: Uuid,
        message_seq: i64,
        sender_user_id: Uuid,
        sender_nickname: &str,
        body_preview: &str,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: "1".to_owned(),
            kind: "direct_message".to_owned(),
            message_id: message_id.to_string(),
            conversation_id: conversation_id.to_string(),
            message_seq: message_seq.to_string(),
            sender_user_id: sender_user_id.to_string(),
            sender_nickname: sender_nickname.chars().take(100).collect(),
            body_preview: body_preview.chars().take(500).collect(),
            created_at: created_at.to_rfc3339(),
        }
    }
}

/// Короткоживущее push-событие о входящем личном звонке.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IncomingCallPush {
    /// Версия схемы data payload.
    pub(crate) schema_version: String,
    /// Машиночитаемый вид события.
    pub(crate) kind: String,
    /// Идентификатор события для дедупликации очереди.
    pub(crate) event_id: String,
    /// Стабильный идентификатор звонка.
    pub(crate) call_id: String,
    /// Идентификатор личного диалога.
    pub(crate) conversation_id: String,
    /// Идентификатор инициатора звонка.
    pub(crate) caller_user_id: String,
    /// Снимок отображаемого имени инициатора.
    pub(crate) caller_nickname: String,
    /// Снимок URL аватара инициатора либо пустая строка.
    pub(crate) caller_avatar_url: String,
    /// RFC 3339 время начала звонка.
    pub(crate) started_at: String,
    /// RFC 3339 момент, после которого входящий звонок нельзя показывать.
    pub(crate) expires_at: String,
}

impl IncomingCallPush {
    pub(crate) fn new(
        call_id: Uuid,
        conversation_id: Uuid,
        caller_user_id: Uuid,
        caller_nickname: &str,
        caller_avatar_url: Option<&str>,
        started_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: "1".to_owned(),
            kind: "incoming_call".to_owned(),
            event_id: call_id.to_string(),
            call_id: call_id.to_string(),
            conversation_id: conversation_id.to_string(),
            caller_user_id: caller_user_id.to_string(),
            caller_nickname: caller_nickname.chars().take(100).collect(),
            caller_avatar_url: caller_avatar_url
                .unwrap_or_default()
                .chars()
                .take(2048)
                .collect(),
            started_at: started_at.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
        }
    }
}

/// Короткоживущее push-событие о завершении личного звонка.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CallEndedPush {
    /// Версия схемы data payload.
    pub(crate) schema_version: String,
    /// Машиночитаемый вид события.
    pub(crate) kind: String,
    /// Отдельный UUID события, чтобы оно не конфликтовало с incoming_call в очереди.
    pub(crate) event_id: String,
    /// Стабильный идентификатор звонка.
    pub(crate) call_id: String,
    /// Причина завершения звонка в snake_case.
    pub(crate) end_reason: String,
    /// RFC 3339 время завершения звонка.
    pub(crate) ended_at: String,
    /// RFC 3339 срок актуальности события.
    pub(crate) expires_at: String,
}

impl CallEndedPush {
    pub(crate) fn new(call_id: Uuid, end_reason: &str, ended_at: DateTime<Utc>) -> Self {
        Self {
            schema_version: "1".to_owned(),
            kind: "call_ended".to_owned(),
            event_id: Uuid::new_v4().to_string(),
            call_id: call_id.to_string(),
            end_reason: end_reason.to_owned(),
            ended_at: ended_at.to_rfc3339(),
            expires_at: (ended_at + Duration::seconds(60)).to_rfc3339(),
        }
    }
}

/// Содержимое push-уведомления о новой заявке в друзья.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FriendRequestPush {
    /// Версия схемы data payload.
    pub(crate) schema_version: String,
    /// Машиночитаемый вид события.
    pub(crate) kind: String,
    /// Идентификатор заявки для дедупликации.
    pub(crate) request_id: String,
    /// Идентификатор отправителя заявки.
    pub(crate) requester_user_id: String,
    /// Отображаемое имя отправителя заявки.
    pub(crate) requester_nickname: String,
    /// RFC 3339 время создания текущей заявки.
    pub(crate) created_at: String,
}

impl FriendRequestPush {
    /// Собирает payload уведомления о новой заявке в друзья.
    pub(crate) fn new(
        request_id: Uuid,
        requester_user_id: Uuid,
        requester_nickname: &str,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema_version: "1".to_owned(),
            kind: "friend_request".to_owned(),
            request_id: request_id.to_string(),
            requester_user_id: requester_user_id.to_string(),
            requester_nickname: requester_nickname.chars().take(100).collect(),
            created_at: created_at.to_rfc3339(),
        }
    }
}

/// Обратно совместимое содержимое задания push-очереди.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum PushPayload {
    /// Новое личное сообщение.
    DirectMessage(DirectMessagePush),
    /// Новая заявка в друзья.
    FriendRequest(FriendRequestPush),
    /// Входящий личный звонок.
    IncomingCall(IncomingCallPush),
    /// Завершение личного звонка.
    CallEnded(CallEndedPush),
}

impl PushPayload {
    /// Возвращает идентификатор события для логов и дедупликации.
    pub(crate) fn event_id(&self) -> &str {
        match self {
            Self::DirectMessage(payload) => &payload.message_id,
            Self::FriendRequest(payload) => &payload.request_id,
            Self::IncomingCall(payload) => &payload.event_id,
            Self::CallEnded(payload) => &payload.event_id,
        }
    }

    /// Возвращает вид события для структурированных логов.
    pub(crate) fn kind(&self) -> &str {
        match self {
            Self::DirectMessage(payload) => &payload.kind,
            Self::FriendRequest(payload) => &payload.kind,
            Self::IncomingCall(payload) => &payload.kind,
            Self::CallEnded(payload) => &payload.kind,
        }
    }

    /// Короткий TTL FCM нужен только звонковым событиям.
    pub(crate) fn fcm_ttl(&self) -> Option<&'static str> {
        match self {
            Self::IncomingCall(_) | Self::CallEnded(_) => Some("60s"),
            Self::DirectMessage(_) | Self::FriendRequest(_) => None,
        }
    }

    /// Не позволяет очереди доставить уже бессмысленное звонковое событие.
    pub(crate) fn is_expired(&self, now: DateTime<Utc>) -> bool {
        let expires_at = match self {
            Self::IncomingCall(payload) => Some(payload.expires_at.as_str()),
            Self::CallEnded(payload) => Some(payload.expires_at.as_str()),
            Self::DirectMessage(_) | Self::FriendRequest(_) => None,
        };
        let Some(expires_at) = expires_at else {
            return false;
        };
        DateTime::parse_from_rfc3339(expires_at)
            .map(|expires_at| expires_at.with_timezone(&Utc) <= now)
            .unwrap_or(true)
    }
}

/// Установка, способная принимать push-уведомления.
#[derive(Debug, Clone)]
pub(crate) struct PushInstallation {
    /// Идентификатор установки.
    pub(crate) id: Uuid,
    /// Идентификатор auth-сессии установки.
    pub(crate) session_id: Uuid,
}

/// Задание постоянной очереди вместе с адресом доставки.
#[derive(Debug, Clone)]
pub(crate) struct PendingDelivery {
    /// Идентификатор задания.
    pub(crate) id: Uuid,
    /// Идентификатор установки.
    pub(crate) installation_id: Uuid,
    /// Идентификатор auth-сессии установки.
    pub(crate) session_id: Uuid,
    /// Непрозрачный FCM-токен.
    pub(crate) token: String,
    /// Число уже выполненных попыток.
    pub(crate) attempts: i32,
    /// Содержимое системного push-уведомления.
    pub(crate) payload: PushPayload,
}

#[cfg(test)]
mod tests {
    use super::{DirectMessagePush, FriendRequestPush, PushPayload, direct_message_preview};
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn direct_message_payload_matches_android_data_contract() {
        let message_id = Uuid::new_v4();
        let conversation_id = Uuid::new_v4();
        let sender_user_id = Uuid::new_v4();
        let created_at = Utc
            .with_ymd_and_hms(2026, 7, 13, 10, 20, 30)
            .single()
            .expect("test timestamp should be valid");
        let payload = DirectMessagePush::new(
            message_id,
            conversation_id,
            42,
            sender_user_id,
            "Alice",
            "Привет",
            created_at,
        );

        assert_eq!(
            serde_json::to_value(PushPayload::DirectMessage(payload))
                .expect("payload should serialize"),
            json!({
                "schema_version": "1",
                "kind": "direct_message",
                "message_id": message_id.to_string(),
                "conversation_id": conversation_id.to_string(),
                "message_seq": "42",
                "sender_user_id": sender_user_id.to_string(),
                "sender_nickname": "Alice",
                "body_preview": "Привет",
                "created_at": created_at.to_rfc3339(),
            })
        );
    }

    #[test]
    fn friend_request_payload_matches_android_data_contract() {
        let request_id = Uuid::new_v4();
        let requester_user_id = Uuid::new_v4();
        let created_at = Utc
            .with_ymd_and_hms(2026, 8, 9, 10, 20, 30)
            .single()
            .expect("test timestamp should be valid");
        let payload = FriendRequestPush::new(request_id, requester_user_id, "Alice", created_at);

        assert_eq!(
            serde_json::to_value(PushPayload::FriendRequest(payload))
                .expect("payload should serialize"),
            json!({
                "schema_version": "1",
                "kind": "friend_request",
                "request_id": request_id.to_string(),
                "requester_user_id": requester_user_id.to_string(),
                "requester_nickname": "Alice",
                "created_at": created_at.to_rfc3339(),
            })
        );
    }

    #[test]
    fn queued_payload_deserializes_both_supported_event_kinds() {
        let direct_message = json!({
            "schema_version": "1",
            "kind": "direct_message",
            "message_id": Uuid::new_v4().to_string(),
            "conversation_id": Uuid::new_v4().to_string(),
            "message_seq": "42",
            "sender_user_id": Uuid::new_v4().to_string(),
            "sender_nickname": "Alice",
            "body_preview": "Привет",
            "created_at": Utc::now().to_rfc3339(),
        });
        let friend_request = json!({
            "schema_version": "1",
            "kind": "friend_request",
            "request_id": Uuid::new_v4().to_string(),
            "requester_user_id": Uuid::new_v4().to_string(),
            "requester_nickname": "Bob",
            "created_at": Utc::now().to_rfc3339(),
        });

        assert!(matches!(
            serde_json::from_value::<PushPayload>(direct_message)
                .expect("direct message payload should deserialize"),
            PushPayload::DirectMessage(_)
        ));
        assert!(matches!(
            serde_json::from_value::<PushPayload>(friend_request)
                .expect("friend request payload should deserialize"),
            PushPayload::FriendRequest(_)
        ));
    }

    #[test]
    fn direct_message_payload_limits_user_visible_strings_by_characters() {
        let payload = DirectMessagePush::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Uuid::new_v4(),
            &"я".repeat(101),
            &"🙂".repeat(501),
            Utc::now(),
        );

        assert_eq!(payload.sender_nickname.chars().count(), 100);
        assert_eq!(payload.body_preview.chars().count(), 500);
    }

    #[test]
    fn image_only_message_has_non_empty_preview() {
        let payload = DirectMessagePush::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Uuid::new_v4(),
            "Alice",
            &direct_message_preview("", true),
            Utc::now(),
        );

        assert_eq!(payload.body_preview, "Изображение");
    }
}
