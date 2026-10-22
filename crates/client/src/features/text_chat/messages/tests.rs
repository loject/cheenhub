use cheenhub_contracts::realtime::TextChatMessage;

use super::apply_message_edit;

fn message() -> TextChatMessage {
    TextChatMessage {
        id: "message-id".to_owned(),
        server_id: "server-id".to_owned(),
        room_id: "room-id".to_owned(),
        author_user_id: "author-id".to_owned(),
        author_nickname: "Автор".to_owned(),
        author_avatar_url: None,
        body: "Исходный текст".to_owned(),
        attachments: Vec::new(),
        delivery_status: None,
        created_at: "2026-10-23T00:00:00Z".to_owned(),
        edited_at: None,
    }
}

#[test]
fn edit_replaces_body_without_moving_message() {
    let first = message();
    let mut messages = vec![first.clone()];
    let mut updated = first;
    updated.body = "Исправлено".to_owned();
    updated.edited_at = Some("2026-10-23T01:00:00Z".to_owned());

    assert!(apply_message_edit(&mut messages, updated));
    assert_eq!(messages[0].body, "Исправлено");
    assert_eq!(messages[0].id, "message-id");
    assert_eq!(messages[0].created_at, "2026-10-23T00:00:00Z");
}

#[test]
fn old_or_different_room_event_does_not_overwrite_message() {
    let mut latest = message();
    latest.body = "Последний вариант".to_owned();
    latest.edited_at = Some("2026-10-23T02:00:00Z".to_owned());
    let mut messages = vec![latest.clone()];

    let mut stale = latest.clone();
    stale.body = "Устаревший вариант".to_owned();
    stale.edited_at = Some("2026-10-23T01:00:00Z".to_owned());
    assert!(!apply_message_edit(&mut messages, stale));
    assert_eq!(messages[0].body, "Последний вариант");

    let mut wrong_room = latest;
    wrong_room.room_id = "another-room".to_owned();
    wrong_room.edited_at = Some("2026-10-23T03:00:00Z".to_owned());
    assert!(!apply_message_edit(&mut messages, wrong_room));
}

#[test]
fn edit_not_in_current_history_is_ignored() {
    let mut messages = vec![message()];
    let mut missing = message();
    missing.id = "another-message-id".to_owned();
    missing.edited_at = Some("2026-10-23T01:00:00Z".to_owned());

    assert!(!apply_message_edit(&mut messages, missing));
    assert_eq!(messages.len(), 1);
}
