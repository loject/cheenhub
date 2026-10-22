use cheenhub_contracts::realtime::TextChatMessage;

use super::{merge_pages, next_cursor};

fn message(id: &str) -> TextChatMessage {
    TextChatMessage {
        id: id.to_owned(),
        server_id: "server".to_owned(),
        room_id: "room".to_owned(),
        author_user_id: "user".to_owned(),
        author_nickname: "Автор".to_owned(),
        author_avatar_url: None,
        body: id.to_owned(),
        attachments: Vec::new(),
        delivery_status: None,
        created_at: "2026-10-14T00:00:00Z".to_owned(),
        edited_at: None,
    }
}

#[test]
fn target_on_loaded_page_stops_pagination_even_if_more_history_exists() {
    let page = vec![message("old"), message("pin")];

    assert_eq!(next_cursor("pin", "recent", &page, true).unwrap(), None);
}

#[test]
fn absent_target_advances_to_oldest_message_on_page() {
    let page = vec![message("old"), message("recent")];

    assert_eq!(
        next_cursor("pin", "cursor", &page, true).unwrap(),
        Some("old".to_owned())
    );
}

#[test]
fn missing_target_at_end_reports_unavailable_message() {
    let page = vec![message("first")];

    assert!(next_cursor("pin", "recent", &page, false).is_err());
}

#[test]
fn repeated_cursor_and_empty_page_cannot_loop_forever() {
    assert!(next_cursor("pin", "same", &[message("same")], true).is_err());
    assert!(next_cursor("pin", "same", &[], true).is_err());
}

#[test]
fn page_merge_preserves_order_and_live_messages_without_duplicates() {
    let pages = vec![
        vec![message("three"), message("four")],
        vec![message("one"), message("two")],
    ];
    let current = vec![message("four"), message("five")];

    let merged = merge_pages(pages, current);
    let ids = merged
        .iter()
        .map(|message| message.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, ["one", "two", "three", "four", "five"]);
}
