use cheenhub_contracts::{
    presence::UserPresence, realtime::PresenceChanged, rest::DmMessageSummary,
};

use super::merge_direct_messages;

#[test]
fn unified_social_presence_updates_friends_only() {
    use crate::features::social::social_list_state::apply_social_presence;
    use cheenhub_contracts::realtime::SocialListEntrySummary;
    use cheenhub_contracts::rest::UserRelationStatus;

    let entry = |user_id: &str, relation| SocialListEntrySummary {
        user_id: user_id.to_owned(),
        nickname: user_id.to_owned(),
        avatar_url: None,
        relation,
        presence: None,
        conversation: None,
        last_message: None,
    };
    let mut entries = vec![
        entry("alice", Some(UserRelationStatus::Friends)),
        entry("bob", None),
    ];
    assert!(apply_social_presence(
        &mut entries,
        &PresenceChanged {
            user_id: "alice".to_owned(),
            presence: UserPresence::Online,
        }
    ));
    assert_eq!(entries[0].presence, Some(UserPresence::Online));
    assert!(!apply_social_presence(
        &mut entries,
        &PresenceChanged {
            user_id: "bob".to_owned(),
            presence: UserPresence::Online,
        }
    ));
    assert!(entries[1].presence.is_none());
}

#[test]
fn next_social_page_appends_without_duplicate_users() {
    use crate::features::social::social_list_state::merge_social_entries;
    use cheenhub_contracts::realtime::SocialListEntrySummary;

    let entry = |user_id: &str| SocialListEntrySummary {
        user_id: user_id.to_owned(),
        nickname: user_id.to_owned(),
        avatar_url: None,
        relation: None,
        presence: None,
        conversation: None,
        last_message: None,
    };
    let merged = merge_social_entries(vec![entry("alice")], vec![entry("alice"), entry("bob")]);
    assert_eq!(
        merged
            .iter()
            .map(|entry| entry.user_id.as_str())
            .collect::<Vec<_>>(),
        vec!["alice", "bob"]
    );
}

#[test]
fn presence_event_during_fetch_wins_over_older_page_snapshot() {
    use crate::features::social::social_list_state::apply_presence_overrides;
    use cheenhub_contracts::realtime::SocialListEntrySummary;
    use cheenhub_contracts::rest::UserRelationStatus;
    use std::collections::HashMap;

    let entry = |user_id: &str, relation, presence| SocialListEntrySummary {
        user_id: user_id.to_owned(),
        nickname: user_id.to_owned(),
        avatar_url: None,
        relation,
        presence,
        conversation: None,
        last_message: None,
    };
    let mut page = vec![
        entry(
            "friend",
            Some(UserRelationStatus::Friends),
            Some(UserPresence::Offline { last_seen_at: None }),
        ),
        entry("stranger", None, None),
    ];
    let received_during_fetch = HashMap::from([
        ("friend".to_owned(), UserPresence::Online),
        ("stranger".to_owned(), UserPresence::Online),
    ]);
    apply_presence_overrides(&mut page, &received_during_fetch);
    assert_eq!(page[0].presence, Some(UserPresence::Online));
    assert!(
        page[1].presence.is_none(),
        "нельзя раскрывать присутствие не-друзей"
    );
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
