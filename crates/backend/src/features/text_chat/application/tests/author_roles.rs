use cheenhub_contracts::realtime::ServerRoleKind;
use uuid::Uuid;

use super::super::author_roles::author_role_ids;
use super::{registered_user, state};

#[tokio::test]
async fn targeted_role_lookup_excludes_unrequested_departed_and_other_server_members() {
    let state = state();
    let owner = registered_user(&state, "lookup_owner", "lookup-owner@example.com").await;
    let active = registered_user(&state, "lookup_active", "lookup-active@example.com").await;
    let departed = registered_user(&state, "lookup_departed", "lookup-departed@example.com").await;
    let elsewhere =
        registered_user(&state, "lookup_elsewhere", "lookup-elsewhere@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let active_id = Uuid::parse_str(&active.user.id).unwrap();
    let departed_id = Uuid::parse_str(&departed.user.id).unwrap();
    let elsewhere_id = Uuid::parse_str(&elsewhere.user.id).unwrap();
    let server = state
        .server_store
        .insert_server(&owner_id, "First".into())
        .await
        .unwrap();
    let other = state
        .server_store
        .insert_server(&elsewhere_id, "Second".into())
        .await
        .unwrap();
    state
        .server_store
        .insert_server_member(&server.id, &active_id)
        .await
        .unwrap();
    state
        .server_store
        .insert_server_member(&server.id, &departed_id)
        .await
        .unwrap();
    state
        .server_store
        .insert_server_member(&other.id, &elsewhere_id)
        .await
        .unwrap();
    state
        .server_store
        .leave_server(&server.id, &departed_id)
        .await
        .unwrap();

    let roles = state
        .server_store
        .active_member_role_ids_for_users(
            &server.id,
            &[active_id, active_id, departed_id, elsewhere_id],
        )
        .await
        .unwrap();
    assert_eq!(
        roles.len(),
        1,
        "only one distinct active member is requested"
    );
    assert_eq!(roles.get(&active_id), Some(&Vec::new()));
    assert!(!roles.contains_key(&departed_id));
    assert!(!roles.contains_key(&elsewhere_id));
    assert!(
        state
            .server_store
            .active_member_role_ids_for_users(&server.id, &[])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn author_roles_include_implicit_owner_member_and_active_custom_role() {
    let state = state();
    let owner = registered_user(&state, "roles_owner", "roles-owner@example.com").await;
    let author = registered_user(&state, "roles_author", "roles-author@example.com").await;
    let outsider = registered_user(&state, "roles_outsider", "roles-outsider@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let outsider_id = Uuid::parse_str(&outsider.user.id).unwrap();
    let server = state
        .server_store
        .insert_server(&owner_id, "Roles".into())
        .await
        .unwrap();
    state
        .server_store
        .insert_server_member(&server.id, &author_id)
        .await
        .unwrap();
    crate::features::servers::application::list_server_roles(
        &state,
        &owner_id,
        cheenhub_contracts::realtime::ListServerRoles {
            server_id: server.id.to_string(),
        },
    )
    .await
    .unwrap();
    let mut roles = state
        .server_store
        .list_server_roles(&server.id)
        .await
        .unwrap();
    let owner_role = roles
        .iter()
        .find(|r| r.kind == ServerRoleKind::Owner)
        .unwrap()
        .id;
    let member_role = roles
        .iter()
        .find(|r| r.kind == ServerRoleKind::Member)
        .unwrap()
        .id;
    let mut custom = roles
        .iter()
        .find(|r| r.kind == ServerRoleKind::Member)
        .unwrap()
        .clone();
    custom.id = Uuid::new_v4();
    custom.kind = ServerRoleKind::Custom;
    custom.name = "Colored".into();
    custom.position = 1;
    let custom_role_id = custom.id;
    roles.push(custom);
    state
        .server_store
        .replace_server_roles(&server.id, roles)
        .await
        .unwrap();
    state
        .server_store
        .assign_server_member_role(&server.id, &author_id, &custom_role_id, &owner_id)
        .await
        .unwrap();

    let resolved = author_role_ids(
        &state,
        &server,
        &[owner_id, author_id, author_id, outsider_id],
    )
    .await
    .unwrap();
    assert_eq!(resolved.len(), 2);
    assert_eq!(
        resolved.get(&owner_id.to_string()),
        Some(&vec![owner_role.to_string(), member_role.to_string()])
    );
    assert_eq!(
        resolved.get(&author_id.to_string()),
        Some(&vec![custom_role_id.to_string(), member_role.to_string()])
    );
    assert!(!resolved.contains_key(&outsider_id.to_string()));
}

#[tokio::test]
async fn history_and_pins_include_only_their_current_active_authors() {
    use super::super::{list_pinned_messages, load_room_history, pin_message, send_message};
    use super::create_server_room;
    use cheenhub_contracts::realtime::{
        ListPinnedMessages, LoadRoomHistory, PinMessage, SendMessage,
    };
    use cheenhub_contracts::rest::ServerRoomKind;

    let state = state();
    let owner = registered_user(&state, "page_owner", "page-owner@example.com").await;
    let author = registered_user(&state, "page_author", "page-author@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let author_id = Uuid::parse_str(&author.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Pages",
        "text",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let server_uuid = Uuid::parse_str(&server_id).unwrap();
    state
        .server_store
        .insert_server_member(&server_uuid, &author_id)
        .await
        .unwrap();
    crate::features::servers::application::list_server_roles(
        &state,
        &owner_id,
        cheenhub_contracts::realtime::ListServerRoles {
            server_id: server_id.clone(),
        },
    )
    .await
    .unwrap();
    for (account, id) in [(&owner, owner_id), (&author, author_id)] {
        send_message(
            &state,
            &account.user,
            &id,
            SendMessage {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                body: "hello".into(),
                attachment_ids: Vec::new(),
            },
        )
        .await
        .unwrap();
    }
    tokio::task::yield_now().await;
    let history = load_room_history(
        &state,
        &owner_id,
        LoadRoomHistory {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            before_message_id: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(history.messages.len(), 2);
    assert_eq!(history.author_role_ids.len(), 2);
    assert!(history.author_role_ids.contains_key(&owner_id.to_string()));
    assert!(history.author_role_ids.contains_key(&author_id.to_string()));

    let author_message_id = history
        .messages
        .iter()
        .find(|m| m.author_user_id == author.user.id)
        .unwrap()
        .id
        .clone();
    pin_message(
        &state,
        &owner_id,
        PinMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            message_id: author_message_id,
        },
    )
    .await
    .unwrap();
    let pins = list_pinned_messages(
        &state,
        &owner_id,
        ListPinnedMessages {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(pins.messages.len(), 1);
    assert_eq!(pins.author_role_ids.len(), 1);
    assert!(pins.author_role_ids.contains_key(&author_id.to_string()));

    state
        .server_store
        .leave_server(&server_uuid, &author_id)
        .await
        .unwrap();
    let history = load_room_history(
        &state,
        &owner_id,
        LoadRoomHistory {
            server_id,
            room_id,
            before_message_id: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(history.messages.len(), 2, "past messages remain visible");
    assert_eq!(
        history.author_role_ids.len(),
        1,
        "departed authors have no color"
    );
    assert!(history.author_role_ids.contains_key(&owner_id.to_string()));
}

#[tokio::test]
async fn targeted_realtime_resolver_enforces_room_access_batch_limit_and_active_membership() {
    use super::super::{TextChatApplicationError, resolve_author_roles};
    use super::create_server_room;
    use cheenhub_contracts::realtime::ResolveAuthorRoles;
    use cheenhub_contracts::rest::ServerRoomKind;

    let state = state();
    let owner = registered_user(&state, "resolve_owner", "resolve-owner@example.com").await;
    let member = registered_user(&state, "resolve_member", "resolve-member@example.com").await;
    let outsider =
        registered_user(&state, "resolve_outsider", "resolve-outsider@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).unwrap();
    let member_id = Uuid::parse_str(&member.user.id).unwrap();
    let outsider_id = Uuid::parse_str(&outsider.user.id).unwrap();
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Resolver",
        "text",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let server_uuid = Uuid::parse_str(&server_id).unwrap();
    state
        .server_store
        .insert_server_member(&server_uuid, &member_id)
        .await
        .unwrap();
    crate::features::servers::application::list_server_roles(
        &state,
        &owner_id,
        cheenhub_contracts::realtime::ListServerRoles {
            server_id: server_id.clone(),
        },
    )
    .await
    .unwrap();
    let request = ResolveAuthorRoles {
        server_id: server_id.clone(),
        room_id: room_id.clone(),
        author_user_ids: vec![
            owner_id.to_string(),
            member_id.to_string(),
            member_id.to_string(),
            outsider_id.to_string(),
        ],
    };
    let resolved = resolve_author_roles(&state, &owner_id, request.clone())
        .await
        .unwrap();
    assert_eq!(resolved.server_id, server_id);
    assert_eq!(resolved.author_role_ids.len(), 2);
    assert!(resolved.author_role_ids.contains_key(&owner_id.to_string()));
    assert!(
        resolved
            .author_role_ids
            .contains_key(&member_id.to_string())
    );
    assert!(
        !resolved
            .author_role_ids
            .contains_key(&outsider_id.to_string())
    );

    assert!(matches!(
        resolve_author_roles(&state, &outsider_id, request.clone()).await,
        Err(TextChatApplicationError::Unauthorized(_))
    ));
    let mut oversized = request.clone();
    oversized.author_user_ids = vec![member_id.to_string(); 101];
    assert!(matches!(
        resolve_author_roles(&state, &owner_id, oversized).await,
        Err(TextChatApplicationError::BadRequest(_))
    ));
    state
        .server_store
        .leave_server(&server_uuid, &member_id)
        .await
        .unwrap();
    let resolved = resolve_author_roles(&state, &owner_id, request)
        .await
        .unwrap();
    assert!(
        !resolved
            .author_role_ids
            .contains_key(&member_id.to_string())
    );
}
