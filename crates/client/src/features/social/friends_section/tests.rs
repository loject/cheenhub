use cheenhub_contracts::rest::{DmLastMessageSummary, FriendSummary};

use super::{friend_message_preview, unread_badge_label};

#[test]
fn unread_badge_caps_only_display_value() {
    assert_eq!(unread_badge_label(0), "0");
    assert_eq!(unread_badge_label(99), "99");
    assert_eq!(unread_badge_label(100000), "99+");
}

#[test]
fn message_preview_marks_own_message_and_image_only_message() {
    let mut friend = FriendSummary {
        user_id: "friend-1".to_owned(),
        nickname: "Friend".to_owned(),
        avatar_url: None,
        unread_count: 0,
        last_message: Some(DmLastMessageSummary {
            id: "message-1".to_owned(),
            sender_user_id: "current-user".to_owned(),
            body: " Ответ ".to_owned(),
            has_image: false,
            created_at: "2026-09-10T00:00:00Z".to_owned(),
        }),
        friends_since: "2026-06-30T00:00:00Z".to_owned(),
    };
    assert_eq!(friend_message_preview(&friend, "current-user"), "Вы: Ответ");

    let message = friend
        .last_message
        .as_mut()
        .expect("last message should exist");
    message.sender_user_id = "friend-1".to_owned();
    message.body.clear();
    message.has_image = true;
    assert_eq!(
        friend_message_preview(&friend, "current-user"),
        "Изображение"
    );
}
