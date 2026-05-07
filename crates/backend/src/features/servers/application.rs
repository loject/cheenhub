//! Server application flows.

use cheenhub_contracts::rest::{
    AcceptServerInviteResponse, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse, ServerInviteSummary,
    ServerRoomKind, ServerRoomSummary, ServerSummary, UpdateServerRoomRequest,
    UpdateServerRoomResponse,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::features::auth::application as auth_application;
use crate::features::auth::error::AuthError;
use crate::features::servers::domain::{Server, ServerRoom};
use crate::features::servers::error::ServerError;
use crate::features::servers::validation;
use crate::state::AppState;

/// Creates a server owned by the current user.
pub(crate) async fn create(
    state: &AppState,
    access_token: &str,
    request: CreateServerRequest,
) -> Result<CreateServerResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let owner_user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let valid = validation::create_server(request.name)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = state
        .server_store
        .insert_server(&owner_user_id, valid.name)
        .await
        .map_err(ServerError::Internal)?;
    state
        .server_store
        .insert_server_member(&server.id, &owner_user_id)
        .await
        .map_err(ServerError::Internal)?;
    state
        .server_store
        .insert_server_room(&server.id, "общий".to_owned(), ServerRoomKind::TextAndVoice)
        .await
        .map_err(ServerError::Internal)?;

    Ok(CreateServerResponse {
        server: server_summary(&server, &owner_user_id, true),
    })
}

/// Lists servers available to the current user.
pub(crate) async fn list(
    state: &AppState,
    access_token: &str,
) -> Result<ListServersResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let servers = state
        .server_store
        .list_servers(&user_id)
        .await
        .map_err(ServerError::Internal)?;

    Ok(ListServersResponse {
        servers: servers
            .iter()
            .map(|access| server_summary(&access.server, &user_id, access.is_member))
            .collect(),
    })
}

/// Creates an invite for a server owned by the current user.
pub(crate) async fn create_invite(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: CreateServerInviteRequest,
) -> Result<CreateServerInviteResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let owner_user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let server_id = Uuid::parse_str(&server_id)
        .map_err(|_| ServerError::BadRequest("Сервер не найден.".to_owned()))?;
    let valid = validation::create_server_invite(request.max_uses, request.expires_in_days)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let Some(server) = state
        .server_store
        .find_owned_server(&server_id, &owner_user_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    };
    let expires_at = valid
        .expires_in_days
        .map(|days| Utc::now() + Duration::days(days.into()));
    let invite = state
        .server_store
        .insert_server_invite(&server.id, &owner_user_id, valid.max_uses, expires_at)
        .await
        .map_err(ServerError::Internal)?;

    Ok(CreateServerInviteResponse {
        code: invite.id.to_string(),
    })
}

/// Loads server invite information for the current user.
pub(crate) async fn invite_info(
    state: &AppState,
    access_token: &str,
    code: String,
) -> Result<ServerInviteInfoResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let code = Uuid::parse_str(&code)
        .map_err(|_| ServerError::BadRequest("Приглашение не найдено.".to_owned()))?;
    let Some(invite) = state
        .server_store
        .find_server_invite(&code)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
    };

    if invite
        .expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now())
    {
        return Err(ServerError::BadRequest(
            "Срок действия приглашения истек.".to_owned(),
        ));
    }

    let Some(server) = state
        .server_store
        .find_server(&invite.server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };
    let uses = state
        .server_store
        .count_server_invite_uses(&invite.id)
        .await
        .map_err(ServerError::Internal)?;
    let is_member = server.owner_user_id == user_id
        || state
            .server_store
            .find_active_server_member(&server.id, &user_id)
            .await
            .map_err(ServerError::Internal)?
            .is_some();

    Ok(ServerInviteInfoResponse {
        invite: ServerInviteSummary {
            code: invite.id.to_string(),
            uses,
            max_uses: invite.max_uses,
            expires_at: invite.expires_at.map(|expires_at| expires_at.to_rfc3339()),
        },
        server: server_summary(&server, &user_id, is_member),
    })
}

/// Accepts a server invite for the current user.
pub(crate) async fn accept_invite(
    state: &AppState,
    access_token: &str,
    code: String,
) -> Result<AcceptServerInviteResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let code = Uuid::parse_str(&code)
        .map_err(|_| ServerError::BadRequest("Приглашение не найдено.".to_owned()))?;
    let Some(invite) = state
        .server_store
        .find_server_invite(&code)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
    };

    if invite
        .expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now())
    {
        return Err(ServerError::BadRequest(
            "Срок действия приглашения истек.".to_owned(),
        ));
    }

    let Some(server) = state
        .server_store
        .find_server(&invite.server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };

    let is_owner = server.owner_user_id == user_id;
    let active_member = state
        .server_store
        .find_active_server_member(&server.id, &user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_some();

    if is_owner || active_member {
        return Ok(AcceptServerInviteResponse {
            server: server_summary(&server, &user_id, true),
            already_member: true,
        });
    }

    let uses = state
        .server_store
        .count_server_invite_uses(&invite.id)
        .await
        .map_err(ServerError::Internal)?;
    if invite.max_uses.is_some_and(|max_uses| uses >= max_uses) {
        return Err(ServerError::BadRequest(
            "Лимит использований приглашения исчерпан.".to_owned(),
        ));
    }

    state
        .server_store
        .insert_server_member(&server.id, &user_id)
        .await
        .map_err(ServerError::Internal)?;
    state
        .server_store
        .insert_server_invite_use(&invite.id, &user_id)
        .await
        .map_err(ServerError::Internal)?;

    Ok(AcceptServerInviteResponse {
        server: server_summary(&server, &user_id, true),
        already_member: false,
    })
}

/// Leaves a server as the current user.
pub(crate) async fn leave(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<(), ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let server_id = Uuid::parse_str(&server_id)
        .map_err(|_| ServerError::BadRequest("Сервер не найден.".to_owned()))?;
    let Some(server) = state
        .server_store
        .find_server(&server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };

    if server.owner_user_id == user_id {
        return Err(ServerError::BadRequest(
            "Владелец сервера не может покинуть сервер.".to_owned(),
        ));
    }

    let active_member = state
        .server_store
        .find_active_server_member(&server.id, &user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_some();
    if !active_member {
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    }

    state
        .server_store
        .leave_server(&server.id, &user_id)
        .await
        .map_err(ServerError::Internal)
}

/// Lists rooms available on a server for the current user.
pub(crate) async fn list_rooms(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<ListServerRoomsResponse, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let server = server_for_member_or_owner(state, &server_id, &user_id).await?;
    let rooms = state
        .server_store
        .list_server_rooms(&server.id)
        .await
        .map_err(ServerError::Internal)?;

    Ok(ListServerRoomsResponse {
        rooms: rooms.iter().map(room_summary).collect(),
    })
}

/// Creates a room on a server owned by the current user.
pub(crate) async fn create_room(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: CreateServerRoomRequest,
) -> Result<CreateServerRoomResponse, ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let valid = validation::server_room(request.name)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = owned_server(state, &server_id, &owner_user_id).await?;
    let room = state
        .server_store
        .insert_server_room(&server.id, valid.name, request.kind)
        .await
        .map_err(ServerError::Internal)?;

    Ok(CreateServerRoomResponse {
        room: room_summary(&room),
    })
}

/// Updates a room on a server owned by the current user.
pub(crate) async fn update_room(
    state: &AppState,
    access_token: &str,
    server_id: String,
    room_id: String,
    request: UpdateServerRoomRequest,
) -> Result<UpdateServerRoomResponse, ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let room_id = Uuid::parse_str(&room_id)
        .map_err(|_| ServerError::BadRequest("Комната не найдена.".to_owned()))?;
    let valid = validation::server_room(request.name)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = owned_server(state, &server_id, &owner_user_id).await?;
    let Some(room) = state
        .server_store
        .update_server_room(&server.id, &room_id, valid.name, request.kind)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Комната не найдена.".to_owned()));
    };

    Ok(UpdateServerRoomResponse {
        room: room_summary(&room),
    })
}

/// Deletes a room from a server owned by the current user.
pub(crate) async fn delete_room(
    state: &AppState,
    access_token: &str,
    server_id: String,
    room_id: String,
) -> Result<(), ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let room_id = Uuid::parse_str(&room_id)
        .map_err(|_| ServerError::BadRequest("Комната не найдена.".to_owned()))?;
    let server = owned_server(state, &server_id, &owner_user_id).await?;
    let Some(room) = state
        .server_store
        .find_server_room(&server.id, &room_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Комната не найдена.".to_owned()));
    };
    let room_count = state
        .server_store
        .count_server_rooms(&server.id)
        .await
        .map_err(ServerError::Internal)?;

    if room_count <= 1 {
        return Err(ServerError::BadRequest(
            "Нельзя удалить последнюю комнату сервера.".to_owned(),
        ));
    }

    state
        .server_store
        .delete_server_room(&server.id, &room.id)
        .await
        .map_err(ServerError::Internal)
}

fn server_summary(server: &Server, user_id: &Uuid, is_member: bool) -> ServerSummary {
    ServerSummary {
        id: server.id.to_string(),
        name: server.name.clone(),
        is_owner: server.owner_user_id == *user_id,
        is_member,
    }
}

fn room_summary(room: &ServerRoom) -> ServerRoomSummary {
    ServerRoomSummary {
        id: room.id.to_string(),
        name: room.name.clone(),
        kind: room.kind,
        position: room.position,
    }
}

async fn current_user_id(state: &AppState, access_token: &str) -> Result<Uuid, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;

    Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))
}

fn parse_server_id(server_id: String) -> Result<Uuid, ServerError> {
    Uuid::parse_str(&server_id).map_err(|_| ServerError::BadRequest("Сервер не найден.".to_owned()))
}

async fn owned_server(
    state: &AppState,
    server_id: &Uuid,
    owner_user_id: &Uuid,
) -> Result<Server, ServerError> {
    state
        .server_store
        .find_owned_server(server_id, owner_user_id)
        .await
        .map_err(ServerError::Internal)?
        .ok_or_else(|| ServerError::NotFound("Сервер не найден или недоступен.".to_owned()))
}

async fn server_for_member_or_owner(
    state: &AppState,
    server_id: &Uuid,
    user_id: &Uuid,
) -> Result<Server, ServerError> {
    let Some(server) = state
        .server_store
        .find_server(server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };

    if server.owner_user_id == *user_id {
        return Ok(server);
    }

    let is_member = state
        .server_store
        .find_active_server_member(&server.id, user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_some();
    if is_member {
        return Ok(server);
    }

    Err(ServerError::NotFound(
        "Сервер не найден или недоступен.".to_owned(),
    ))
}

fn map_auth_error(error: AuthError) -> ServerError {
    match error {
        AuthError::BadRequest(message) | AuthError::Unauthorized(message) => {
            ServerError::Unauthorized(message)
        }
        AuthError::Conflict(message) => ServerError::BadRequest(message),
        AuthError::Internal(error) => ServerError::Internal(error),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cheenhub_contracts::rest::{
        CreateServerInviteRequest, CreateServerRequest, CreateServerRoomRequest, RegisterRequest,
        ServerRoomKind, UpdateServerRoomRequest,
    };

    use super::{
        accept_invite, create, create_invite, create_room, delete_room, invite_info, leave, list,
        list_rooms, update_room,
    };
    use crate::features::auth::application as auth_application;
    use crate::features::auth::infrastructure::InMemoryAuthStore;
    use crate::features::auth::security::keys::AuthKeys;
    use crate::features::servers::error::ServerError;
    use crate::features::servers::infrastructure::{InMemoryServerStore, ServerStore};
    use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
    use crate::realtime::hub::RealtimeHub;
    use crate::state::AppState;
    use uuid::Uuid;

    fn state() -> AppState {
        state_with_store(Arc::new(InMemoryServerStore::default()))
    }

    fn state_with_store(server_store: Arc<InMemoryServerStore>) -> AppState {
        AppState {
            auth_store: Arc::new(InMemoryAuthStore::default()),
            server_store,
            text_chat_store: Arc::new(InMemoryTextChatStore::default()),
            realtime_hub: Arc::new(RealtimeHub::default()),
            auth_keys: AuthKeys::generate_for_tests(),
            access_token_lifetime_minutes: 15,
            refresh_token_lifetime_days: 30,
        }
    }

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

    #[tokio::test]
    async fn owner_can_create_server_invite() {
        let server_store = Arc::new(InMemoryServerStore::default());
        let state = state_with_store(server_store.clone());
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "invite_owner".to_owned(),
                email: "invite-owner@example.com".to_owned(),
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
                name: "Invite Hub".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");

        let response = create_invite(
            &state,
            &auth.access_token,
            server.server.id.clone(),
            CreateServerInviteRequest {
                max_uses: Some(5),
                expires_in_days: Some(3),
            },
        )
        .await
        .expect("invite creation should succeed");
        let invites = server_store
            .invites_for_tests()
            .expect("invites should be readable");

        assert_eq!(invites.len(), 1);
        assert_eq!(response.code, invites[0].id.to_string());
        assert_eq!(invites[0].server_id.to_string(), server.server.id);
        assert_eq!(invites[0].creator_user_id.to_string(), auth.user.id);
        assert_eq!(invites[0].max_uses, Some(5));
        assert!(invites[0].expires_at.is_some());
        assert!(invites[0].created_at <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn owner_can_load_server_invite_info() {
        let state = state();
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "info_owner".to_owned(),
                email: "info-owner@example.com".to_owned(),
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
                name: "Info Hub".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");
        let invite = create_invite(
            &state,
            &auth.access_token,
            server.server.id.clone(),
            CreateServerInviteRequest {
                max_uses: Some(7),
                expires_in_days: Some(5),
            },
        )
        .await
        .expect("invite creation should succeed");

        let response = invite_info(&state, &auth.access_token, invite.code.clone())
            .await
            .expect("invite info should load");

        assert_eq!(response.invite.code, invite.code);
        assert_eq!(response.invite.uses, 0);
        assert_eq!(response.invite.max_uses, Some(7));
        assert!(response.invite.expires_at.is_some());
        assert_eq!(response.server.id, server.server.id);
        assert_eq!(response.server.name, "Info Hub");
        assert!(response.server.is_owner);
        assert!(response.server.is_member);
    }

    #[tokio::test]
    async fn non_owner_can_load_server_invite_info() {
        let state = state();
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "info_owner_two".to_owned(),
                email: "info-owner-two@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "info_guest".to_owned(),
                email: "info-guest@example.com".to_owned(),
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
                name: "Shared Info".to_owned(),
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
        .expect("invite creation should succeed");

        let response = invite_info(&state, &guest_auth.access_token, invite.code)
            .await
            .expect("invite info should load for another user");

        assert_eq!(response.server.id, server.server.id);
        assert!(!response.server.is_owner);
        assert!(!response.server.is_member);
    }

    #[tokio::test]
    async fn non_member_accepts_invite_and_server_appears_in_list() {
        let server_store = Arc::new(InMemoryServerStore::default());
        let state = state_with_store(server_store.clone());
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "accept_owner".to_owned(),
                email: "accept-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "accept_guest".to_owned(),
                email: "accept-guest@example.com".to_owned(),
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
                name: "Joinable".to_owned(),
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
        .expect("invite creation should succeed");

        let accepted = accept_invite(&state, &guest_auth.access_token, invite.code.clone())
            .await
            .expect("invite should be accepted");
        let listed = list(&state, &guest_auth.access_token)
            .await
            .expect("joined server list should load");
        let invite_info = invite_info(&state, &guest_auth.access_token, invite.code)
            .await
            .expect("invite info should load");
        let invite_uses = server_store
            .invite_uses_for_tests()
            .expect("invite uses should be readable");

        assert!(!accepted.already_member);
        assert_eq!(accepted.server.id, server.server.id);
        assert!(!accepted.server.is_owner);
        assert!(accepted.server.is_member);
        assert_eq!(listed.servers, vec![accepted.server]);
        assert_eq!(invite_info.invite.uses, 1);
        assert_eq!(invite_uses.len(), 1);
        assert_eq!(invite_uses[0].user_id.to_string(), guest_auth.user.id);
    }

    #[tokio::test]
    async fn member_can_leave_joined_server() {
        let state = state();
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "leave_owner".to_owned(),
                email: "leave-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "leave_guest".to_owned(),
                email: "leave-guest@example.com".to_owned(),
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
                name: "Leavable".to_owned(),
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
        .expect("invite creation should succeed");
        accept_invite(&state, &guest_auth.access_token, invite.code)
            .await
            .expect("invite should be accepted");

        leave(&state, &guest_auth.access_token, server.server.id)
            .await
            .expect("member should leave");
        let listed = list(&state, &guest_auth.access_token)
            .await
            .expect("server list should load after leaving");

        assert!(listed.servers.is_empty());
    }

    #[tokio::test]
    async fn owner_cannot_leave_owned_server() {
        let state = state();
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "leave_blocked_owner".to_owned(),
                email: "leave-blocked-owner@example.com".to_owned(),
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
                name: "Owned".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");

        let error = leave(&state, &auth.access_token, server.server.id)
            .await
            .expect_err("owner leave should fail");

        assert!(matches!(error, ServerError::BadRequest(_)));
    }

    #[tokio::test]
    async fn active_member_accept_returns_already_member_without_new_usage() {
        let server_store = Arc::new(InMemoryServerStore::default());
        let state = state_with_store(server_store.clone());
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "already_owner".to_owned(),
                email: "already-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let server = create(
            &state,
            &owner_auth.access_token,
            CreateServerRequest {
                name: "Already".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");
        let invite = create_invite(
            &state,
            &owner_auth.access_token,
            server.server.id,
            CreateServerInviteRequest {
                max_uses: Some(1),
                expires_in_days: None,
            },
        )
        .await
        .expect("invite creation should succeed");

        let accepted = accept_invite(&state, &owner_auth.access_token, invite.code)
            .await
            .expect("owner should already be a member");
        let invite_uses = server_store
            .invite_uses_for_tests()
            .expect("invite uses should be readable");

        assert!(accepted.already_member);
        assert!(accepted.server.is_owner);
        assert!(accepted.server.is_member);
        assert!(invite_uses.is_empty());
    }

    #[tokio::test]
    async fn accept_invite_accepts_compact_uuid_code() {
        let state = state();
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "compact_accept_owner".to_owned(),
                email: "compact-accept-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "compact_accept_guest".to_owned(),
                email: "compact-accept-guest@example.com".to_owned(),
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
                name: "Compact Accept".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");
        let invite = create_invite(
            &state,
            &owner_auth.access_token,
            server.server.id,
            CreateServerInviteRequest {
                max_uses: None,
                expires_in_days: None,
            },
        )
        .await
        .expect("invite creation should succeed");

        let response = accept_invite(
            &state,
            &guest_auth.access_token,
            invite.code.replace('-', ""),
        )
        .await
        .expect("compact invite should be accepted");

        assert_eq!(response.server.name, "Compact Accept");
        assert!(response.server.is_member);
    }

    #[tokio::test]
    async fn invite_info_accepts_compact_uuid_code() {
        let state = state();
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "compact_owner".to_owned(),
                email: "compact-owner@example.com".to_owned(),
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
                name: "Compact".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");
        let invite = create_invite(
            &state,
            &auth.access_token,
            server.server.id,
            CreateServerInviteRequest {
                max_uses: None,
                expires_in_days: None,
            },
        )
        .await
        .expect("invite creation should succeed");
        let compact_code = invite.code.replace('-', "");

        let response = invite_info(&state, &auth.access_token, compact_code)
            .await
            .expect("compact invite code should load");

        assert_eq!(response.invite.code, invite.code);
    }

    #[tokio::test]
    async fn invite_info_rejects_missing_invalid_and_expired_invites() {
        let server_store = Arc::new(InMemoryServerStore::default());
        let state = state_with_store(server_store.clone());
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "invite_error_owner".to_owned(),
                email: "invite-error-owner@example.com".to_owned(),
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
                name: "Expired".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");
        let owner_user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
        let server_id = Uuid::parse_str(&server.server.id).expect("server id should be uuid");
        let expired_invite = server_store
            .insert_server_invite(
                &server_id,
                &owner_user_id,
                None,
                Some(chrono::Utc::now() - chrono::Duration::days(1)),
            )
            .await
            .expect("expired invite should be inserted");

        let invalid = invite_info(&state, &auth.access_token, "not-a-uuid".to_owned())
            .await
            .expect_err("invalid invite code should fail");
        let missing = invite_info(&state, &auth.access_token, Uuid::new_v4().to_string())
            .await
            .expect_err("missing invite should fail");
        let expired = invite_info(&state, &auth.access_token, expired_invite.id.to_string())
            .await
            .expect_err("expired invite should fail");

        assert!(matches!(invalid, ServerError::BadRequest(_)));
        assert!(matches!(missing, ServerError::NotFound(_)));
        assert!(matches!(expired, ServerError::BadRequest(_)));
    }

    #[tokio::test]
    async fn accept_invite_rejects_missing_invalid_expired_and_exhausted_invites() {
        let server_store = Arc::new(InMemoryServerStore::default());
        let state = state_with_store(server_store.clone());
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "accept_error_owner".to_owned(),
                email: "accept-error-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let first_guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "accept_error_first_guest".to_owned(),
                email: "accept-error-first-guest@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("first guest registration should succeed");
        let second_guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "accept_error_second_guest".to_owned(),
                email: "accept-error-second-guest@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("second guest registration should succeed");
        let server = create(
            &state,
            &owner_auth.access_token,
            CreateServerRequest {
                name: "Accept Errors".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");
        let server_id = Uuid::parse_str(&server.server.id).expect("server id should be uuid");
        let owner_user_id = Uuid::parse_str(&owner_auth.user.id).expect("user id should be uuid");
        let expired_invite = server_store
            .insert_server_invite(
                &server_id,
                &owner_user_id,
                None,
                Some(chrono::Utc::now() - chrono::Duration::days(1)),
            )
            .await
            .expect("expired invite should be inserted");
        let limited_invite = create_invite(
            &state,
            &owner_auth.access_token,
            server.server.id,
            CreateServerInviteRequest {
                max_uses: Some(1),
                expires_in_days: None,
            },
        )
        .await
        .expect("limited invite should be created");

        accept_invite(
            &state,
            &first_guest_auth.access_token,
            limited_invite.code.clone(),
        )
        .await
        .expect("first use should succeed");

        let invalid = accept_invite(
            &state,
            &second_guest_auth.access_token,
            "not-a-uuid".to_owned(),
        )
        .await
        .expect_err("invalid invite code should fail");
        let missing = accept_invite(
            &state,
            &second_guest_auth.access_token,
            Uuid::new_v4().to_string(),
        )
        .await
        .expect_err("missing invite should fail");
        let expired = accept_invite(
            &state,
            &second_guest_auth.access_token,
            expired_invite.id.to_string(),
        )
        .await
        .expect_err("expired invite should fail");
        let exhausted = accept_invite(&state, &second_guest_auth.access_token, limited_invite.code)
            .await
            .expect_err("exhausted invite should fail");

        assert!(matches!(invalid, ServerError::BadRequest(_)));
        assert!(matches!(missing, ServerError::NotFound(_)));
        assert!(matches!(expired, ServerError::BadRequest(_)));
        assert!(matches!(exhausted, ServerError::BadRequest(_)));
    }

    #[tokio::test]
    async fn member_can_join_again_after_soft_leave() {
        let server_store = Arc::new(InMemoryServerStore::default());
        let state = state_with_store(server_store.clone());
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "rejoin_owner".to_owned(),
                email: "rejoin-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "rejoin_guest".to_owned(),
                email: "rejoin-guest@example.com".to_owned(),
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
                name: "Rejoin".to_owned(),
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
        .expect("invite creation should succeed");
        let server_id = Uuid::parse_str(&server.server.id).expect("server id should be uuid");
        let guest_user_id = Uuid::parse_str(&guest_auth.user.id).expect("user id should be uuid");

        accept_invite(&state, &guest_auth.access_token, invite.code.clone())
            .await
            .expect("first join should succeed");
        server_store
            .leave_server(&server_id, &guest_user_id)
            .await
            .expect("test member should soft leave");
        let second_join = accept_invite(&state, &guest_auth.access_token, invite.code)
            .await
            .expect("second join should succeed");
        let guest_members = server_store
            .members_for_tests()
            .expect("members should be readable")
            .into_iter()
            .filter(|member| member.server_id == server_id && member.user_id == guest_user_id)
            .collect::<Vec<_>>();
        let invite_uses = server_store
            .invite_uses_for_tests()
            .expect("invite uses should be readable");

        assert!(!second_join.already_member);
        assert_eq!(guest_members.len(), 2);
        assert_eq!(invite_uses.len(), 2);
    }

    #[tokio::test]
    async fn non_owner_cannot_create_server_invite() {
        let state = state();
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "owner_user".to_owned(),
                email: "owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "guest_user".to_owned(),
                email: "guest@example.com".to_owned(),
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
                name: "Private".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");

        let error = create_invite(
            &state,
            &guest_auth.access_token,
            server.server.id,
            CreateServerInviteRequest {
                max_uses: None,
                expires_in_days: None,
            },
        )
        .await
        .expect_err("non-owner invite creation should fail");

        assert!(matches!(
            error,
            crate::features::servers::error::ServerError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn create_invite_rejects_invalid_settings() {
        let state = state();
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "invalid_invite_owner".to_owned(),
                email: "invalid-invite-owner@example.com".to_owned(),
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
                name: "Validation".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");

        assert!(
            create_invite(
                &state,
                &auth.access_token,
                server.server.id.clone(),
                CreateServerInviteRequest {
                    max_uses: Some(0),
                    expires_in_days: None,
                },
            )
            .await
            .is_err()
        );
        assert!(
            create_invite(
                &state,
                &auth.access_token,
                server.server.id,
                CreateServerInviteRequest {
                    max_uses: None,
                    expires_in_days: Some(366),
                },
            )
            .await
            .is_err()
        );
    }
}
