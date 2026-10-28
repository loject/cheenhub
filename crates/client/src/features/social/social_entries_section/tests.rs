//! Предпросмотр и индикаторы объединённого списка друзей и собеседников.

use cheenhub_contracts::realtime::SocialListEntrySummary;
use cheenhub_contracts::rest::{DmLastMessageSummary, UserRelationStatus};

use super::{social_message_preview, unread_badge_label};

#[test]
fn unified_list_preview_keeps_sender_and_image_semantics() {
    let mut entry = SocialListEntrySummary {
        user_id: "friend".to_owned(),
        nickname: "Друг".to_owned(),
        avatar_url: None,
        relation: Some(UserRelationStatus::Friends),
        presence: None,
        conversation: None,
        last_message: Some(DmLastMessageSummary {
            id: "message".to_owned(),
            sender_user_id: "myself".to_owned(),
            body: " Ответ ".to_owned(),
            has_image: false,
            created_at: "2026-10-28T00:00:00Z".to_owned(),
        }),
    };
    assert_eq!(social_message_preview(&entry, "myself"), "Вы: Ответ");
    let message = entry.last_message.as_mut().unwrap();
    message.sender_user_id = "friend".to_owned();
    message.body.clear();
    message.has_image = true;
    assert_eq!(social_message_preview(&entry, "myself"), "Изображение");
    entry.relation = None;
    entry.last_message = None;
    assert_eq!(social_message_preview(&entry, "myself"), "Начните общение");
}

#[test]
fn unified_list_badge_caps_display_not_the_counter() {
    assert_eq!(unread_badge_label(0), "0");
    assert_eq!(unread_badge_label(99), "99");
    assert_eq!(unread_badge_label(1024), "99+");
}
