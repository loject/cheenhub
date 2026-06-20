//! Voice chat presence application flows.

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, KickVoiceMember, LeaveVoiceRoom, ListServerVoiceRooms, RealtimeKind,
    RealtimeModule, ServerRoleKind, ServerRolePermission, ServerVoiceRoomsSnapshot, VoiceChatKind,
    VoiceRoomParticipant, VoiceRoomSnapshot,
};
use cheenhub_contracts::rest::{AuthUser, ServerRoomKind};
use chrono::Utc;
use uuid::Uuid;

use crate::features::voice_chat::infrastructure::VoicePresence;
use crate::state::AppState;

mod avatar;

pub(crate) use avatar::update_user_avatar;

/// Входит в одну комнату с поддержкой голоса и возвращает текущий снимок участников.
pub(crate) async fn join_room(
    state: &AppState,
    realtime_stream_id: Uuid,
    session_id: Uuid,
    user: &AuthUser,
    user_id: &Uuid,
    request: JoinVoiceRoom,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    ensure_room_voice_available(state, user_id, &server_id, &room_id).await?;
    let removed = state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id,
            session_id,
            server_id,
            room_id,
            user_id: *user_id,
            nickname: user.nickname.clone(),
            avatar_url: user.avatar_url.clone(),
            joined_at: Utc::now(),
        })
        .await;

    fanout_removed_rooms(state, removed, Some((server_id, room_id))).await;
    let snapshot = room_snapshot(state, &server_id, &room_id).await;
    fanout_snapshot(state, snapshot.clone()).await;

    Ok(snapshot)
}

/// Покидает одну комнату с поддержкой голоса и возвращает текущий снимок участников.
pub(crate) async fn leave_room(
    state: &AppState,
    realtime_stream_id: Uuid,
    user_id: &Uuid,
    request: LeaveVoiceRoom,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let removed = state
        .voice_presence_store
        .leave_room(&realtime_stream_id, &server_id, &room_id)
        .await;

    if removed.is_empty() {
        ensure_room_voice_available(state, user_id, &server_id, &room_id).await?;
        return Ok(room_snapshot(state, &server_id, &room_id).await);
    }

    let snapshot = room_snapshot(state, &server_id, &room_id).await;
    fanout_snapshot(state, snapshot.clone()).await;

    Ok(snapshot)
}

/// Кикает одного участника из голосовой комнаты, если у запрашивающего пользователя есть право.
pub(crate) async fn kick_member(
    state: &AppState,
    kicker_user_id: &Uuid,
    request: KickVoiceMember,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let target_user_id = parse_id(&request.user_id, "Пользователь не найден.")?;

    if *kicker_user_id == target_user_id {
        return Err(VoiceChatApplicationError::BadRequest(
            "Нельзя кикнуть самого себя.".to_owned(),
        ));
    }

    if !user_can_kick_voice(state, kicker_user_id, &server_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
    {
        return Err(VoiceChatApplicationError::Unauthorized(
            "Недостаточно прав для кика из голосовой комнаты.".to_owned(),
        ));
    }

    let removed = state
        .voice_presence_store
        .kick_user_from_room(&target_user_id, &server_id, &room_id)
        .await;

    if removed.is_empty() {
        return Err(VoiceChatApplicationError::NotFound(
            "Пользователь не находится в этой голосовой комнате.".to_owned(),
        ));
    }

    let snapshot = room_snapshot(state, &server_id, &room_id).await;
    fanout_snapshot(state, snapshot.clone()).await;

    Ok(snapshot)
}

/// Перечисляет активные снимки участников голосовых комнат одного сервера.
pub(crate) async fn list_server_voice_rooms(
    state: &AppState,
    user_id: &Uuid,
    request: ListServerVoiceRooms,
) -> Result<ServerVoiceRoomsSnapshot, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    if !user_has_server_access(state, user_id, &server_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
    {
        return Err(VoiceChatApplicationError::Unauthorized(
            "Нет доступа к этому серверу.".to_owned(),
        ));
    }

    let rooms = state
        .voice_presence_store
        .server_room_participants(&server_id)
        .await
        .into_iter()
        .map(|(room_id, participants)| VoiceRoomSnapshot {
            server_id: server_id.to_string(),
            room_id: room_id.to_string(),
            participants: participants.iter().map(participant_summary).collect(),
        })
        .collect::<Vec<_>>();

    tracing::debug!(
        server_id = %server_id,
        active_voice_rooms = rooms.len(),
        "listed server voice room participants"
    );

    Ok(ServerVoiceRoomsSnapshot {
        server_id: server_id.to_string(),
        rooms,
    })
}

/// Удаляет присутствие, принадлежащее закрытому realtime-потоку.
pub(crate) async fn disconnect_realtime_stream(state: &AppState, realtime_stream_id: Uuid) {
    let removed = state
        .voice_presence_store
        .leave_realtime_stream(&realtime_stream_id)
        .await;
    fanout_removed_rooms(state, removed, None).await;
}

/// Обновляет активные снимки голосового присутствия после изменения никнейма профиля.
pub(crate) async fn update_user_nickname(state: &AppState, user_id: &Uuid, nickname: String) {
    let rooms = state
        .voice_presence_store
        .update_user_nickname(user_id, nickname)
        .await;
    if rooms.is_empty() {
        return;
    }

    tracing::info!(
        user_id = %user_id,
        rooms = rooms.len(),
        "updated active voice presence nickname"
    );
    for (server_id, room_id) in rooms {
        let snapshot = room_snapshot(state, &server_id, &room_id).await;
        fanout_snapshot(state, snapshot).await;
    }
}

/// Ошибка приложения голосового чата.
#[derive(Debug)]
pub(crate) enum VoiceChatApplicationError {
    /// Request shape or target is invalid.
    BadRequest(String),
    /// User cannot access the requested voice room.
    Unauthorized(String),
    /// Resource was not found.
    NotFound(String),
    /// Unexpected internal failure.
    Internal(anyhow::Error),
}

async fn ensure_room_voice_available(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
    room_id: &Uuid,
) -> Result<(), VoiceChatApplicationError> {
    let Some(room) = state
        .server_store
        .find_server_room(server_id, room_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
    else {
        return Err(VoiceChatApplicationError::NotFound(
            "Комната не найдена.".to_owned(),
        ));
    };
    if room.kind == ServerRoomKind::Text {
        return Err(VoiceChatApplicationError::BadRequest(
            "В этой комнате нет голосового чата.".to_owned(),
        ));
    }
    if user_has_server_access(state, user_id, server_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
    {
        Ok(())
    } else {
        Err(VoiceChatApplicationError::Unauthorized(
            "Нет доступа к этой комнате.".to_owned(),
        ))
    }
}

async fn user_can_kick_voice(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(server) = state.server_store.find_server(server_id).await? else {
        return Ok(false);
    };
    if server.owner_user_id == *user_id {
        return Ok(true);
    }
    if state
        .server_store
        .find_active_server_member(server_id, user_id)
        .await?
        .is_none()
    {
        return Ok(false);
    }

    let roles = state.server_store.list_server_roles(server_id).await?;
    let member_roles = state
        .server_store
        .list_server_member_roles(server_id)
        .await?;
    let user_role_ids: Vec<_> = member_roles
        .iter()
        .filter(|(uid, _)| uid == user_id)
        .map(|(_, rid)| *rid)
        .collect();

    Ok(roles.iter().any(|role| {
        (role.kind == ServerRoleKind::Member || user_role_ids.contains(&role.id))
            && role
                .permissions
                .contains(&ServerRolePermission::KickVoiceMembers)
    }))
}

async fn user_has_server_access(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(server) = state.server_store.find_server(server_id).await? else {
        return Ok(false);
    };
    if server.owner_user_id == *user_id {
        return Ok(true);
    }

    Ok(state
        .server_store
        .find_active_server_member(server_id, user_id)
        .await?
        .is_some())
}

async fn fanout_removed_rooms(
    state: &AppState,
    removed: Vec<VoicePresence>,
    skip: Option<(Uuid, Uuid)>,
) {
    let mut rooms = Vec::<(Uuid, Uuid)>::new();
    for presence in removed {
        let room = (presence.server_id, presence.room_id);
        if Some(room) == skip || rooms.contains(&room) {
            continue;
        }
        rooms.push(room);
    }

    for (server_id, room_id) in rooms {
        let snapshot = room_snapshot(state, &server_id, &room_id).await;
        fanout_snapshot(state, snapshot).await;
    }
}

async fn room_snapshot(state: &AppState, server_id: &Uuid, room_id: &Uuid) -> VoiceRoomSnapshot {
    let participants = state
        .voice_presence_store
        .room_participants(server_id, room_id)
        .await
        .iter()
        .map(participant_summary)
        .collect();

    VoiceRoomSnapshot {
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        participants,
    }
}

async fn fanout_snapshot(state: &AppState, snapshot: VoiceRoomSnapshot) {
    let Ok(server_id) = Uuid::parse_str(&snapshot.server_id) else {
        return;
    };
    let recipients = state
        .realtime_hub
        .recipients(state, RealtimeModule::VoiceChat, &server_id)
        .await;
    let stream_ids = recipients
        .iter()
        .map(|recipient| recipient.stream_id)
        .collect::<Vec<_>>();
    tracing::debug!(
        server_id = %snapshot.server_id,
        room_id = %snapshot.room_id,
        participants = snapshot.participants.len(),
        recipients = stream_ids.len(),
        "fanning out voice room participants changed event"
    );

    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::VoiceChat,
            &server_id,
            RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged),
            &stream_ids,
            snapshot,
        )
        .await;
}

fn parse_id(value: &str, message: &str) -> Result<Uuid, VoiceChatApplicationError> {
    Uuid::parse_str(value).map_err(|_| VoiceChatApplicationError::BadRequest(message.to_owned()))
}

fn participant_summary(presence: &VoicePresence) -> VoiceRoomParticipant {
    VoiceRoomParticipant {
        user_id: presence.user_id.to_string(),
        nickname: presence.nickname.clone(),
        avatar_url: presence.avatar_url.clone(),
        joined_at: presence.joined_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests;
