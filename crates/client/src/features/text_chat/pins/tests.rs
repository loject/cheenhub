use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeKind, RealtimeModule, TextChatKind};

use super::{RoomPinEvent, decode_pin_event};

#[test]
fn pinned_event_decodes_with_message_and_room_identity() {
    let event = RealtimeEnvelope::new(
        RealtimeModule::TextChat,
        RealtimeKind::TextChat(TextChatKind::MessagePinned),
        None,
        serde_json::json!({
            "message": {
                "id": "message-1",
                "server_id": "server-1",
                "room_id": "room-1",
                "author_user_id": "author-1",
                "author_nickname": "Автор",
                "author_avatar_url": null,
                "body": "Текст закрепления",
                "attachments": [],
                "created_at": "2026-10-14T00:00:00Z"
            },
            "pinned_by_user_id": "moderator-1",
            "pinned_at": "2026-10-14T00:00:01Z"
        }),
    )
    .unwrap();

    let Some(RoomPinEvent::Pinned(pinned)) = decode_pin_event(event) else {
        panic!("expected pinned message event");
    };
    assert_eq!(pinned.message.id, "message-1");
    assert_eq!(pinned.message.room_id, "room-1");
    assert_eq!(pinned.pinned_by_user_id, "moderator-1");
}

#[test]
fn unpinned_event_decodes_message_identity() {
    let event = RealtimeEnvelope::new(
        RealtimeModule::TextChat,
        RealtimeKind::TextChat(TextChatKind::MessageUnpinned),
        None,
        serde_json::json!({
            "server_id": "server-1", "room_id": "room-1", "message_id": "message-2"
        }),
    )
    .unwrap();

    let Some(RoomPinEvent::Unpinned(unpinned)) = decode_pin_event(event) else {
        panic!("expected unpinned message event");
    };
    assert_eq!(unpinned.message_id, "message-2");
    assert_eq!(unpinned.room_id, "room-1");
}

#[test]
fn pin_decoder_ignores_other_modules_and_invalid_payloads() {
    let unrelated = RealtimeEnvelope::new(
        RealtimeModule::Social,
        RealtimeKind::TextChat(TextChatKind::MessagePinned),
        None,
        serde_json::json!({}),
    )
    .unwrap();
    let invalid = RealtimeEnvelope::new(
        RealtimeModule::TextChat,
        RealtimeKind::TextChat(TextChatKind::MessagePinned),
        None,
        serde_json::json!({"message_id": "missing-required-data"}),
    )
    .unwrap();

    assert!(decode_pin_event(unrelated).is_none());
    assert!(decode_pin_event(invalid).is_none());
}
