use super::*;

fn message(id: &str, timestamp: &str) -> TextChatMessage {
    TextChatMessage {
        id: id.to_owned(),
        server_id: "server".to_owned(),
        room_id: "room".to_owned(),
        author_user_id: "author".to_owned(),
        author_nickname: "Автор".to_owned(),
        author_avatar_url: None,
        body: id.to_owned(),
        attachments: Vec::new(),
        delivery_status: None,
        created_at: timestamp.to_owned(),
    }
}

fn messages(ids: &[&str]) -> Vec<TextChatMessage> {
    ids.iter()
        .map(|id| message(id, "2026-10-02T00:00:00Z"))
        .collect()
}

fn page(ids: &[&str], has_more: bool) -> RoomHistory {
    RoomHistory {
        server_id: "server".to_owned(),
        room_id: "room".to_owned(),
        messages: messages(ids),
        has_more,
    }
}

fn ids(messages: Vec<TextChatMessage>) -> Vec<String> {
    messages.into_iter().map(|message| message.id).collect()
}

#[test]
fn reconnect_fetches_gap_and_keeps_loaded_pages_with_equal_timestamps() {
    let mut refresh = HistoryRefresh::new(messages(&["02", "03", "04"]));
    assert_eq!(
        refresh.push_page(page(&["07", "08"], true)),
        Some("07".to_owned())
    );
    assert_eq!(
        refresh.push_page(page(&["05", "06"], true)),
        Some("05".to_owned())
    );
    assert_eq!(
        refresh.push_page(page(&["01", "02", "03", "04"], true)),
        None
    );
    let (merged, has_more) = refresh.finish(messages(&["02", "03", "04", "09"]));
    assert_eq!(
        ids(merged),
        ["02", "03", "04", "05", "06", "07", "08", "09"]
    );
    assert!(has_more);
}

#[test]
fn reconnect_removes_messages_deleted_while_hidden() {
    let saved = messages(&["02", "03", "04"]);
    let mut refresh = HistoryRefresh::new(saved.clone());
    assert_eq!(refresh.push_page(page(&["01", "04", "05"], false)), None);
    let (merged, has_more) = refresh.finish(saved);
    assert_eq!(ids(merged), ["04", "05"]);
    assert!(has_more);
}

#[test]
fn reconnect_handles_empty_room_without_looping() {
    let mut refresh = HistoryRefresh::new(messages(&["02", "03"]));
    assert_eq!(refresh.push_page(page(&[], false)), None);
    let (merged, has_more) = refresh.finish(messages(&["02", "03"]));
    assert!(merged.is_empty());
    assert!(!has_more);
}

#[test]
fn reconnect_uses_actual_timestamp_order_across_timezones() {
    let oldest = message("02", "2026-10-02T01:00:00+01:00");
    let newer = message("03", "2026-10-02T00:00:01Z");
    let mut refresh = HistoryRefresh::new(vec![oldest]);
    assert_eq!(
        refresh.push_page(RoomHistory {
            server_id: "server".to_owned(),
            room_id: "room".to_owned(),
            messages: vec![newer],
            has_more: true
        }),
        Some("03".to_owned())
    );
}

#[test]
fn gap_message_deleted_during_refresh_is_not_resurrected() {
    let mut refresh = HistoryRefresh::new(messages(&["02"]));
    assert_eq!(
        refresh.push_page(page(&["04", "05"], true)),
        Some("04".to_owned())
    );
    refresh.record_deletion("04".to_owned());
    assert_eq!(refresh.push_page(page(&["01", "02", "03"], false)), None);
    let (merged, _) = refresh.finish(messages(&["02", "04"]));
    assert_eq!(ids(merged), ["02", "03", "05"]);
}
