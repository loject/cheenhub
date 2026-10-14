use cheenhub_contracts::realtime::{PinnedTextMessage, TextChatMessage};

use super::PinPreview;

#[test]
fn text_pin_preview_keeps_message_identity_and_formats_timestamp() {
    let pin = PinnedTextMessage {
        message: TextChatMessage {
            id: "message-id".to_owned(),
            server_id: "server-id".to_owned(),
            room_id: "room-id".to_owned(),
            author_user_id: "user-id".to_owned(),
            author_nickname: "Участник".to_owned(),
            author_avatar_url: None,
            body: "Важная информация".to_owned(),
            attachments: Vec::new(),
            delivery_status: None,
            created_at: "2026-10-14T10:00:00Z".to_owned(),
        },
        pinned_by_user_id: "moderator-id".to_owned(),
        pinned_at: "2026-10-14T12:34:00+00:00".to_owned(),
    };

    let preview = PinPreview::from(&pin);

    assert_eq!(preview.message_id, "message-id");
    assert_eq!(preview.author, "Участник");
    assert_eq!(preview.body, "Важная информация");
    assert_eq!(preview.pinned_at, "14.10.2026 12:34");
    assert_eq!(preview.pinned_at_iso, "2026-10-14T12:34:00+00:00");
}
