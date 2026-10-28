use cheenhub_contracts::{
    presence::UserPresence,
    realtime::PresenceChanged,
    rest::{DmMessageSummary, FriendSummary},
};

use super::{apply_friend_presence, merge_direct_messages};

#[test]
fn presence_event_updates_only_the_matching_friend() {
    let friend = |user_id: &str| FriendSummary {
        user_id: user_id.to_owned(),
        nickname: user_id.to_owned(),
        avatar_url: None,
        unread_count: 3,
        last_message: None,
        friends_since: "2026-10-20T00:00:00Z".to_owned(),
        presence: UserPresence::Offline { last_seen_at: None },
    };
    let mut friends = vec![friend("alice"), friend("bob")];
    let changed = PresenceChanged {
        user_id: "bob".to_owned(),
        presence: UserPresence::Online,
    };

    assert!(apply_friend_presence(&mut friends, &changed));
    assert_eq!(
        friends[0].presence,
        UserPresence::Offline { last_seen_at: None }
    );
    assert_eq!(friends[1].presence, UserPresence::Online);
    assert_eq!(friends[1].unread_count, 3);
    assert!(!apply_friend_presence(
        &mut friends,
        &PresenceChanged {
            user_id: "unknown".to_owned(),
            presence: UserPresence::Online,
        },
    ));
}

fn message(id: &str, seq: i64, body: &str) -> DmMessageSummary {
    DmMessageSummary {
        id: id.to_owned(),
        conversation_id: "conversation".to_owned(),
        seq,
        sender_user_id: "alice".to_owned(),
        sender_nickname: "Alice".to_owned(),
        sender_avatar_url: None,
        body: body.to_owned(),
        image: None,
        delivery_status: None,
        created_at: "2026-10-23T00:00:00Z".to_owned(),
        edited_at: None,
    }
}

#[test]
fn refresh_keeps_older_pages_and_applies_edits_without_new_animation() {
    let mut changed = message("2", 2, "Исправлено");
    changed.edited_at = Some("2026-10-23T00:10:00Z".to_owned());
    let (merged, added) = merge_direct_messages(
        vec![
            message("1", 1, "Старая страница"),
            message("2", 2, "До правки"),
        ],
        vec![changed],
    );
    assert!(added.is_empty());
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].body, "Старая страница");
    assert_eq!(merged[1].body, "Исправлено");
    assert!(merged[1].edited_at.is_some());
}
