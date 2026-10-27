use super::*;

#[test]
fn legacy_history_and_pins_default_to_empty_author_roles() {
    let history: RoomHistory = serde_json::from_value(serde_json::json!({
        "server_id": "server", "room_id": "room", "messages": [], "has_more": false
    }))
    .unwrap();
    assert!(history.author_role_ids.is_empty());
    assert!(
        serde_json::to_value(&history)
            .unwrap()
            .get("author_role_ids")
            .is_none()
    );

    let pins: PinnedMessages = serde_json::from_value(serde_json::json!({
        "server_id": "server", "room_id": "room", "messages": [], "limit": 50
    }))
    .unwrap();
    assert!(pins.author_role_ids.is_empty());
    assert!(
        serde_json::to_value(&pins)
            .unwrap()
            .get("author_role_ids")
            .is_none()
    );
}

#[test]
fn author_roles_are_deduplicated_outside_individual_message_payloads() {
    let message = serde_json::json!({
        "id": "message", "server_id": "server", "room_id": "room",
        "author_user_id": "author", "author_nickname": "Nick",
        "author_avatar_url": null, "body": "hello", "created_at": "2026-10-27T00:00:00Z"
    });
    let history: RoomHistory = serde_json::from_value(serde_json::json!({
        "server_id": "server", "room_id": "room",
        "messages": [message.clone(), message.clone()], "has_more": false,
        "author_role_ids": { "author": ["role"] }
    }))
    .unwrap();
    let encoded = serde_json::to_value(&history).unwrap();
    assert_eq!(
        encoded["author_role_ids"]["author"],
        serde_json::json!(["role"])
    );
    assert!(encoded["messages"][0].get("author_role_ids").is_none());
    assert!(encoded["messages"][1].get("author_role_ids").is_none());

    let pins: PinnedMessages = serde_json::from_value(serde_json::json!({
        "server_id": "server", "room_id": "room", "limit": 50,
        "messages": [{ "message": message, "pinned_by_user_id": "moderator",
            "pinned_at": "2026-10-27T00:00:00Z" }],
        "author_role_ids": { "author": ["role"] }
    }))
    .unwrap();
    let encoded = serde_json::to_value(&pins).unwrap();
    assert_eq!(
        encoded["author_role_ids"]["author"],
        serde_json::json!(["role"])
    );
    assert!(
        encoded["messages"][0]["message"]
            .get("author_role_ids")
            .is_none()
    );
}

#[test]
fn author_role_resolver_round_trips_as_a_bounded_request_envelope() {
    let request = ResolveAuthorRoles {
        server_id: "server".to_owned(),
        room_id: "room".to_owned(),
        author_user_ids: vec!["author".to_owned()],
    };
    let encoded = serde_json::to_value(&request).unwrap();
    let decoded: ResolveAuthorRoles = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, request);
    let response = AuthorRolesResolved {
        server_id: "server".to_owned(),
        author_role_ids: std::collections::HashMap::from([(
            "author".to_owned(),
            vec!["colored".to_owned()],
        )]),
    };
    assert_eq!(
        serde_json::from_value::<AuthorRolesResolved>(serde_json::to_value(&response).unwrap())
            .unwrap(),
        response
    );
    assert_eq!(
        serde_json::to_value(TextChatKind::ResolveAuthorRoles).unwrap(),
        "resolve_author_roles"
    );
    assert_eq!(
        serde_json::to_value(TextChatKind::AuthorRolesResolved).unwrap(),
        "author_roles_resolved"
    );
}

#[test]
fn server_role_invalidation_identifies_server_and_optional_target_user() {
    let change = AuthorRolesChanged {
        server_id: "server".to_owned(),
        user_id: Some("author".to_owned()),
        roles: None,
    };
    assert_eq!(
        serde_json::from_value::<AuthorRolesChanged>(serde_json::to_value(&change).unwrap())
            .unwrap(),
        change
    );
    assert_eq!(
        serde_json::to_value(TextChatKind::AuthorRolesChanged).unwrap(),
        "author_roles_changed"
    );
}
