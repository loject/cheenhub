use cheenhub_contracts::realtime::{
    AssignServerMemberRole, ListServerRoles, SaveServerRoles, ServerRoleDraft, ServerRoleEntry,
    ServerRoleKind, ServerRolePermission,
};
use cheenhub_contracts::rest::AuthResponse;
use cheenhub_contracts::rest::{
    CreateServerInviteRequest, CreateServerInviteResponse, CreateServerRequest,
    CreateServerRoomRequest, CreateServerRoomResponse, RegisterRequest, ServerRoomKind,
    ServerRoomSummary, ServerRoomWriteAccess, ServerRoomWriteAccessMode, ServerSummary,
    UpdateServerRoomRequest, UpdateServerRoomResponse,
};

use super::*;

/// Регистрирует пользователя с уникальным email и возвращает его сессию.
async fn register(state: &AppState, nickname: &str, email: &str) -> AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed")
}

/// Создает сервер и возвращает его сводку.
async fn create_server(state: &AppState, owner: &AuthResponse, name: &str) -> ServerSummary {
    create(
        state,
        &owner.access_token,
        CreateServerRequest {
            name: name.to_owned(),
        },
    )
    .await
    .expect("server creation should succeed")
    .server
}

/// Приглашает пользователя на сервер.
async fn join_server(
    state: &AppState,
    owner: &AuthResponse,
    guest: &AuthResponse,
    server_id: &str,
) {
    let invite: CreateServerInviteResponse = create_invite(
        state,
        &owner.access_token,
        server_id.to_owned(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    accept_invite(state, &guest.access_token, invite.code)
        .await
        .expect("invite acceptance should succeed");
}

/// Возвращает идентификатор пользователя сессии.
fn user_id(auth: &AuthResponse) -> Uuid {
    Uuid::parse_str(&auth.user.id).expect("user id should be uuid")
}

/// Создает пользовательскую роль на сервере и возвращает ее идентификатор.
async fn create_custom_role(
    state: &AppState,
    owner: &AuthResponse,
    server_id: &str,
    name: &str,
) -> String {
    let saved = save_server_roles(
        state,
        &user_id(owner),
        SaveServerRoles {
            server_id: server_id.to_owned(),
            roles: vec![ServerRoleDraft {
                role_id: None,
                name: name.to_owned(),
                color: "#a855f7".to_owned(),
                kind: ServerRoleKind::Custom,
                permissions: Vec::new(),
            }],
        },
    )
    .await
    .expect("roles should be saved");

    custom_role_id(&saved.roles, name)
}

fn custom_role_id(roles: &[ServerRoleEntry], name: &str) -> String {
    roles
        .iter()
        .find(|role| role.name == name)
        .map(|role| role.role_id.clone())
        .expect("custom role should exist")
}

/// Назначает пользовательскую роль участнику сервера.
async fn assign_role(
    state: &AppState,
    owner: &AuthResponse,
    server_id: &str,
    target: &AuthResponse,
    role_id: &str,
) {
    assign_server_member_role(
        state,
        &user_id(owner),
        AssignServerMemberRole {
            server_id: server_id.to_owned(),
            user_id: target.user.id.clone(),
            role_id: role_id.to_owned(),
        },
    )
    .await
    .expect("role should be assigned");
}

/// Создает комнату с заданным доступом к записи.
async fn create_room_with_access(
    state: &AppState,
    owner: &AuthResponse,
    server_id: &str,
    name: &str,
    write_access: ServerRoomWriteAccess,
) -> CreateServerRoomResponse {
    create_room(
        state,
        &owner.access_token,
        server_id.to_owned(),
        CreateServerRoomRequest {
            name: name.to_owned(),
            kind: ServerRoomKind::TextAndVoice,
            write_access,
        },
    )
    .await
    .expect("room creation should succeed")
}

/// Находит комнату по имени в ответе списка комнат.
fn room_by_name<'a>(rooms: &'a [ServerRoomSummary], name: &str) -> &'a ServerRoomSummary {
    rooms
        .iter()
        .find(|room| room.name == name)
        .expect("room should be listed")
}

#[tokio::test]
async fn default_room_allows_every_member_to_write() {
    let state = state();
    let owner = register(
        &state,
        "write_default_owner",
        "write-default-owner@example.com",
    )
    .await;
    let guest = register(
        &state,
        "write_default_guest",
        "write-default-guest@example.com",
    )
    .await;
    let server = create_server(&state, &owner, "Default Access").await;
    join_server(&state, &owner, &guest, &server.id).await;

    let rooms = list_rooms(&state, &guest.access_token, server.id)
        .await
        .expect("rooms should load");

    assert_eq!(
        room_by_name(&rooms.rooms, "общий").write_access.mode,
        ServerRoomWriteAccessMode::AllMembers
    );
    assert!(room_by_name(&rooms.rooms, "общий").can_write);
}

#[tokio::test]
async fn room_write_access_is_limited_to_selected_roles() {
    let state = state();
    let owner = register(&state, "write_roles_owner", "write-roles-owner@example.com").await;
    let writer = register(
        &state,
        "write_roles_writer",
        "write-roles-writer@example.com",
    )
    .await;
    let reader = register(
        &state,
        "write_roles_reader",
        "write-roles-reader@example.com",
    )
    .await;
    let server = create_server(&state, &owner, "Role Access").await;
    join_server(&state, &owner, &writer, &server.id).await;
    join_server(&state, &owner, &reader, &server.id).await;
    let role_id = create_custom_role(&state, &owner, &server.id, "Редакторы").await;
    assign_role(&state, &owner, &server.id, &writer, &role_id).await;

    let created = create_room_with_access(
        &state,
        &owner,
        &server.id,
        "редакторская",
        ServerRoomWriteAccess {
            mode: ServerRoomWriteAccessMode::SelectedRoles,
            role_ids: vec![role_id],
        },
    )
    .await;

    let writer_rooms = list_rooms(&state, &writer.access_token, server.id.clone())
        .await
        .expect("writer rooms should load");
    let reader_rooms = list_rooms(&state, &reader.access_token, server.id.clone())
        .await
        .expect("reader rooms should load");
    let owner_rooms = list_rooms(&state, &owner.access_token, server.id)
        .await
        .expect("owner rooms should load");

    assert_eq!(
        created.room.write_access.mode,
        ServerRoomWriteAccessMode::SelectedRoles
    );
    assert!(room_by_name(&owner_rooms.rooms, "редакторская").can_write);
    assert!(room_by_name(&writer_rooms.rooms, "редакторская").can_write);
    assert!(!room_by_name(&reader_rooms.rooms, "редакторская").can_write);
    assert!(room_by_name(&reader_rooms.rooms, "общий").can_write);
}

#[tokio::test]
async fn selected_roles_without_roster_lets_only_owner_write() {
    let state = state();
    let owner = register(
        &state,
        "write_locked_owner",
        "write-locked-owner@example.com",
    )
    .await;
    let guest = register(
        &state,
        "write_locked_guest",
        "write-locked-guest@example.com",
    )
    .await;
    let server = create_server(&state, &owner, "Locked Access").await;
    join_server(&state, &owner, &guest, &server.id).await;

    create_room_with_access(
        &state,
        &owner,
        &server.id,
        "только владелец",
        ServerRoomWriteAccess {
            mode: ServerRoomWriteAccessMode::SelectedRoles,
            role_ids: Vec::new(),
        },
    )
    .await;

    let guest_rooms = list_rooms(&state, &guest.access_token, server.id.clone())
        .await
        .expect("guest rooms should load");
    let owner_rooms = list_rooms(&state, &owner.access_token, server.id)
        .await
        .expect("owner rooms should load");

    assert!(!room_by_name(&guest_rooms.rooms, "только владелец").can_write);
    assert!(room_by_name(&owner_rooms.rooms, "только владелец").can_write);
}

#[tokio::test]
async fn room_write_access_rejects_role_from_another_server() {
    let state = state();
    let owner = register(
        &state,
        "write_foreign_owner",
        "write-foreign-owner@example.com",
    )
    .await;
    let other_owner = register(
        &state,
        "write_foreign_other",
        "write-foreign-other@example.com",
    )
    .await;
    let server = create_server(&state, &owner, "Own Roles").await;
    let other_server = create_server(&state, &other_owner, "Foreign Roles").await;
    let foreign_role_id = create_custom_role(&state, &other_owner, &other_server.id, "Чужая").await;

    let error = create_room(
        &state,
        &owner.access_token,
        server.id,
        CreateServerRoomRequest {
            name: "Ошибка".to_owned(),
            kind: ServerRoomKind::Text,
            write_access: ServerRoomWriteAccess {
                mode: ServerRoomWriteAccessMode::SelectedRoles,
                role_ids: vec![foreign_role_id],
            },
        },
    )
    .await
    .expect_err("foreign role should be rejected");

    assert!(matches!(error, ServerError::BadRequest(_)));
}

#[tokio::test]
async fn room_write_access_deduplicates_roles_on_update() {
    let state = state();
    let owner = register(&state, "write_dedup_owner", "write-dedup-owner@example.com").await;
    let server = create_server(&state, &owner, "Dedup Access").await;
    let role_id = create_custom_role(&state, &owner, &server.id, "Модераторы").await;

    let created = create_room_with_access(
        &state,
        &owner,
        &server.id,
        "модераторская",
        ServerRoomWriteAccess::default(),
    )
    .await;
    let updated: UpdateServerRoomResponse = update_room(
        &state,
        &owner.access_token,
        server.id,
        created.room.id,
        UpdateServerRoomRequest {
            name: "модераторская".to_owned(),
            kind: ServerRoomKind::TextAndVoice,
            write_access: ServerRoomWriteAccess {
                mode: ServerRoomWriteAccessMode::SelectedRoles,
                role_ids: vec![role_id.clone(), role_id],
            },
        },
    )
    .await
    .expect("room update should succeed");

    assert_eq!(
        updated.room.write_access.mode,
        ServerRoomWriteAccessMode::SelectedRoles
    );
    assert_eq!(updated.room.write_access.role_ids.len(), 1);
}

#[tokio::test]
async fn manage_rooms_permission_allows_member_to_create_rooms() {
    let state = state();
    let owner = register(
        &state,
        "manage_rooms_owner",
        "manage-rooms-owner@example.com",
    )
    .await;
    let manager = register(&state, "manage_rooms_user", "manage-rooms-user@example.com").await;
    let server = create_server(&state, &owner, "Managed Rooms").await;
    join_server(&state, &owner, &manager, &server.id).await;
    let role_id = create_custom_role(&state, &owner, &server.id, "Управляющие").await;
    assign_role(&state, &owner, &server.id, &manager, &role_id).await;

    let denied = create_room(
        &state,
        &manager.access_token,
        server.id.clone(),
        CreateServerRoomRequest {
            name: "До прав".to_owned(),
            kind: ServerRoomKind::Text,
            write_access: ServerRoomWriteAccess::default(),
        },
    )
    .await
    .expect_err("room creation without permission should fail");
    assert!(matches!(denied, ServerError::NotFound(_)));

    let roles = list_server_roles(
        &state,
        &user_id(&owner),
        ListServerRoles {
            server_id: server.id.clone(),
        },
    )
    .await
    .expect("roles should load");
    let drafts = roles
        .roles
        .iter()
        .map(|role| ServerRoleDraft {
            role_id: Some(role.role_id.clone()),
            name: role.name.clone(),
            color: role.color.clone(),
            kind: role.kind,
            permissions: if role.role_id == role_id {
                vec![ServerRolePermission::ManageRooms]
            } else {
                role.permissions.clone()
            },
        })
        .collect();
    save_server_roles(
        &state,
        &user_id(&owner),
        SaveServerRoles {
            server_id: server.id.clone(),
            roles: drafts,
        },
    )
    .await
    .expect("roles should be saved");

    let created = create_room(
        &state,
        &manager.access_token,
        server.id.clone(),
        CreateServerRoomRequest {
            name: "управляемая".to_owned(),
            kind: ServerRoomKind::Text,
            write_access: ServerRoomWriteAccess::default(),
        },
    )
    .await
    .expect("room with manage permission should succeed");

    assert_eq!(created.room.name, "управляемая");
}
