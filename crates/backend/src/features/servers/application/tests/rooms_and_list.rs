use super::*;

#[tokio::test]
async fn creates_and_lists_servers_for_current_user() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "cheenhero".to_owned(),
            email: "hero@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");

    let created = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "  Dev Server  ".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let listed = list(&state, &auth.access_token)
        .await
        .expect("server list should succeed");

    assert_eq!(created.server.name, "Dev Server");
    assert_eq!(listed.servers, vec![created.server]);
}

#[tokio::test]
async fn new_server_has_default_room() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "room_owner".to_owned(),
            email: "room-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Rooms".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let rooms = list_rooms(&state, &auth.access_token, server.server.id)
        .await
        .expect("room list should load");

    assert_eq!(rooms.rooms.len(), 1);
    assert_eq!(rooms.rooms[0].name, "общий");
    assert_eq!(rooms.rooms[0].kind, ServerRoomKind::TextAndVoice);
    assert_eq!(rooms.rooms[0].position, 0);
}

#[tokio::test]
async fn active_member_can_list_rooms_but_non_member_cannot() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "rooms_access_owner".to_owned(),
            email: "rooms-access-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "rooms_access_guest".to_owned(),
            email: "rooms-access-guest@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("guest registration should succeed");
    let outsider_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "rooms_access_outsider".to_owned(),
            email: "rooms-access-outsider@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("outsider registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Readable Rooms".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite should be created");

    let denied = list_rooms(
        &state,
        &outsider_auth.access_token,
        server.server.id.clone(),
    )
    .await
    .expect_err("outsider should not list rooms");
    accept_invite(&state, &guest_auth.access_token, invite.code)
        .await
        .expect("guest should join");
    let rooms = list_rooms(&state, &guest_auth.access_token, server.server.id)
        .await
        .expect("member should list rooms");

    assert!(matches!(denied, ServerError::NotFound(_)));
    assert_eq!(rooms.rooms.len(), 1);
}

#[tokio::test]
async fn owner_can_create_update_and_delete_room() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "room_crud_owner".to_owned(),
            email: "room-crud-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Crud Rooms".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let created = create_room(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        CreateServerRoomRequest {
            name: "  x  ".to_owned(),
            kind: ServerRoomKind::Text,
        },
    )
    .await
    .expect("room creation should succeed");
    let updated = update_room(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        created.room.id.clone(),
        UpdateServerRoomRequest {
            name: "Voice".to_owned(),
            kind: ServerRoomKind::Voice,
        },
    )
    .await
    .expect("room update should succeed");
    delete_room(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        updated.room.id.clone(),
    )
    .await
    .expect("room deletion should succeed");
    let rooms = list_rooms(&state, &auth.access_token, server.server.id)
        .await
        .expect("room list should load");

    assert_eq!(created.room.name, "x");
    assert_eq!(created.room.kind, ServerRoomKind::Text);
    assert_eq!(created.room.position, 1);
    assert_eq!(updated.room.name, "Voice");
    assert_eq!(updated.room.kind, ServerRoomKind::Voice);
    assert_eq!(rooms.rooms.len(), 1);
    assert_eq!(rooms.rooms[0].name, "общий");
}

#[tokio::test]
async fn non_owner_member_cannot_mutate_rooms() {
    let state = state();
    let owner_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "room_mutation_owner".to_owned(),
            email: "room-mutation-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("owner registration should succeed");
    let guest_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "room_mutation_guest".to_owned(),
            email: "room-mutation-guest@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("guest registration should succeed");
    let server = create(
        &state,
        &owner_auth.access_token,
        CreateServerRequest {
            name: "Locked Rooms".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let invite = create_invite(
        &state,
        &owner_auth.access_token,
        server.server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite should be created");
    accept_invite(&state, &guest_auth.access_token, invite.code)
        .await
        .expect("guest should join");
    let rooms = list_rooms(&state, &guest_auth.access_token, server.server.id.clone())
        .await
        .expect("member should list rooms");
    let room_id = rooms.rooms[0].id.clone();

    let create_error = create_room(
        &state,
        &guest_auth.access_token,
        server.server.id.clone(),
        CreateServerRoomRequest {
            name: "Denied".to_owned(),
            kind: ServerRoomKind::Text,
        },
    )
    .await
    .expect_err("member room creation should fail");
    let update_error = update_room(
        &state,
        &guest_auth.access_token,
        server.server.id.clone(),
        room_id.clone(),
        UpdateServerRoomRequest {
            name: "Denied".to_owned(),
            kind: ServerRoomKind::Voice,
        },
    )
    .await
    .expect_err("member room update should fail");
    let delete_error = delete_room(&state, &guest_auth.access_token, server.server.id, room_id)
        .await
        .expect_err("member room deletion should fail");

    assert!(matches!(create_error, ServerError::NotFound(_)));
    assert!(matches!(update_error, ServerError::NotFound(_)));
    assert!(matches!(delete_error, ServerError::NotFound(_)));
}

#[tokio::test]
async fn cannot_delete_last_room() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "last_room_owner".to_owned(),
            email: "last-room-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Last Room".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");
    let rooms = list_rooms(&state, &auth.access_token, server.server.id.clone())
        .await
        .expect("room list should load");

    let error = delete_room(
        &state,
        &auth.access_token,
        server.server.id,
        rooms.rooms[0].id.clone(),
    )
    .await
    .expect_err("last room deletion should fail");

    assert!(matches!(error, ServerError::BadRequest(_)));
}

#[tokio::test]
async fn room_flows_reject_invalid_ids_and_names() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "invalid_room_owner".to_owned(),
            email: "invalid-room-owner@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed");
    let server = create(
        &state,
        &auth.access_token,
        CreateServerRequest {
            name: "Invalid Rooms".to_owned(),
        },
    )
    .await
    .expect("server creation should succeed");

    let invalid_server_id = list_rooms(&state, &auth.access_token, "not-a-uuid".to_owned())
        .await
        .expect_err("invalid server id should fail");
    let invalid_room_id = update_room(
        &state,
        &auth.access_token,
        server.server.id.clone(),
        "not-a-uuid".to_owned(),
        UpdateServerRoomRequest {
            name: "Room".to_owned(),
            kind: ServerRoomKind::Text,
        },
    )
    .await
    .expect_err("invalid room id should fail");
    let invalid_name = create_room(
        &state,
        &auth.access_token,
        server.server.id,
        CreateServerRoomRequest {
            name: " ".to_owned(),
            kind: ServerRoomKind::Text,
        },
    )
    .await
    .expect_err("invalid room name should fail");

    assert!(matches!(invalid_server_id, ServerError::BadRequest(_)));
    assert!(matches!(invalid_room_id, ServerError::BadRequest(_)));
    assert!(matches!(invalid_name, ServerError::BadRequest(_)));
}

#[tokio::test]
async fn lists_only_current_users_servers() {
    let state = state();
    let first_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "first_user".to_owned(),
            email: "first@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("first registration should succeed");
    let second_auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "second_user".to_owned(),
            email: "second@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("second registration should succeed");

    let first_server = create(
        &state,
        &first_auth.access_token,
        CreateServerRequest {
            name: "First".to_owned(),
        },
    )
    .await
    .expect("first server should be created");
    create(
        &state,
        &second_auth.access_token,
        CreateServerRequest {
            name: "Second".to_owned(),
        },
    )
    .await
    .expect("second server should be created");

    let listed = list(&state, &first_auth.access_token)
        .await
        .expect("server list should succeed");

    assert_eq!(listed.servers, vec![first_server.server]);
}

#[tokio::test]
async fn list_rejects_invalid_access_token() {
    let state = state();

    assert!(list(&state, "not-a-token").await.is_err());
}
