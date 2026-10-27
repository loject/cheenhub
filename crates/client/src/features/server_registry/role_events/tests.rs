use cheenhub_contracts::realtime::{
    AuthorRolesChanged, RealtimeEnvelope, RealtimeKind, RealtimeModule, TextChatKind,
};

use super::decode_author_role_change;

#[test]
fn decodes_targeted_role_invalidation_on_text_chat_stream() {
    let change = AuthorRolesChanged {
        server_id: "server".to_owned(),
        user_id: Some("author".to_owned()),
        roles: None,
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::TextChat,
        RealtimeKind::TextChat(TextChatKind::AuthorRolesChanged),
        None,
        change.clone(),
    )
    .unwrap();
    assert_eq!(decode_author_role_change(envelope), Some(change));
}

#[test]
fn ignores_unrelated_text_chat_envelopes() {
    let change = AuthorRolesChanged {
        server_id: "server".to_owned(),
        user_id: None,
        roles: None,
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::TextChat,
        RealtimeKind::TextChat(TextChatKind::MessageCreated),
        None,
        change,
    )
    .unwrap();
    assert!(decode_author_role_change(envelope).is_none());
}
