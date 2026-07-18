//! Модель данных системных push-уведомлений.

use chrono::{DateTime, Utc};
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
    /// Payload личного сообщения.
    pub(crate) payload: DirectMessagePush,
}

#[cfg(test)]
mod tests {
    use super::{DirectMessagePush, direct_message_preview};
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
            serde_json::to_value(payload).expect("payload should serialize"),
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
