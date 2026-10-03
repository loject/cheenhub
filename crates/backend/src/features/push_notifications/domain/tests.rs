//! Проверки моделей системных push-уведомлений.

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
