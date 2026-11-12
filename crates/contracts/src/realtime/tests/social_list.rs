//! Проверки wire-формата объединённого списка друзей и личных диалогов.

use super::*;
use crate::presence::UserPresence;
use crate::rest::{DmConversationSummary, DmLastMessageSummary, UserRelationStatus};

#[test]
fn social_list_request_round_trips_search_and_cursor_in_realtime_envelope() {
    let request = ListSocialEntries {
        query: Some("али".to_owned()),
        cursor: Some("page-cursor".to_owned()),
        limit: Some(50),
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::Social,
        RealtimeKind::Social(SocialKind::ListSocialEntries),
        Some(Uuid::new_v4()),
        request.clone(),
    )
    .expect("request serializes");

    assert!(envelope.has_matching_module_kind());
    assert_eq!(
        serde_json::to_value(envelope.kind).unwrap(),
        "list_social_entries"
    );
    assert_eq!(envelope.payload["query"], "али");
    assert_eq!(envelope.payload["cursor"], "page-cursor");
    assert_eq!(envelope.payload["limit"], 50);
    let decoded: ListSocialEntries = serde_json::from_value(envelope.payload).unwrap();
    assert_eq!(decoded, request);
}

#[test]
fn social_list_request_accepts_omitted_optional_filters() {
    let request: ListSocialEntries = serde_json::from_value(serde_json::json!({})).unwrap();

    assert_eq!(request.query, None);
    assert_eq!(request.cursor, None);
    assert_eq!(request.limit, None);
}

#[test]
fn social_list_response_preserves_friend_without_dialog_and_nonfriend_with_dialog() {
    let conversation = DmConversationSummary {
        id: "dialog-1".to_owned(),
        friend_user_id: "outside-1".to_owned(),
        friend_nickname: "Борис".to_owned(),
        friend_avatar_url: None,
        relation: None,
        block_status: crate::rest::DmBlockStatus::None,
        unread_count: 3,
        last_read_message_id: None,
        last_read_seq: 0,
        last_read_at: None,
        updated_at: "2026-10-28T12:00:00Z".to_owned(),
    };
    let mut legacy = serde_json::to_value(&conversation).unwrap();
    legacy.as_object_mut().unwrap().remove("block_status");
    let decoded_legacy: DmConversationSummary = serde_json::from_value(legacy).unwrap();
    assert_eq!(
        decoded_legacy.block_status,
        crate::rest::DmBlockStatus::None
    );
    let response = SocialEntries {
        entries: vec![
            SocialListEntrySummary {
                user_id: "friend-1".to_owned(),
                nickname: "Алиса".to_owned(),
                avatar_url: None,
                relation: Some(UserRelationStatus::Friends),
                presence: Some(UserPresence::Online),
                conversation: None,
                last_message: None,
            },
            SocialListEntrySummary {
                user_id: "outside-1".to_owned(),
                nickname: "Борис".to_owned(),
                avatar_url: None,
                relation: None,
                presence: None,
                conversation: Some(conversation),
                last_message: Some(DmLastMessageSummary {
                    id: "message-1".to_owned(),
                    sender_user_id: "outside-1".to_owned(),
                    body: "Привет".to_owned(),
                    has_image: false,
                    created_at: "2026-10-28T12:00:00Z".to_owned(),
                }),
            },
        ],
        next_cursor: Some("next-page".to_owned()),
        has_more: true,
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::Social,
        RealtimeKind::Social(SocialKind::SocialEntries),
        Some(Uuid::new_v4()),
        response.clone(),
    )
    .expect("response serializes");

    assert!(envelope.has_matching_module_kind());
    assert_eq!(
        serde_json::to_value(envelope.kind).unwrap(),
        "social_entries"
    );
    assert_eq!(envelope.payload["entries"][0]["relation"], "friends");
    assert!(envelope.payload["entries"][0]["conversation"].is_null());
    assert!(envelope.payload["entries"][1]["relation"].is_null());
    assert_eq!(
        envelope.payload["entries"][1]["conversation"]["unread_count"],
        3
    );
    assert_eq!(
        envelope.payload["entries"][1]["last_message"]["body"],
        "Привет"
    );
    let decoded: SocialEntries = serde_json::from_value(envelope.payload).unwrap();
    assert_eq!(decoded, response);
}
