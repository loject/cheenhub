//! Voice chat presence application flows.

use cheenhub_contracts::realtime::{
    DirectMessageVoiceRoomsSnapshot, JoinDirectMessageVoiceRoom, JoinVoiceRoom, KickVoiceMember,
    LeaveDirectMessageVoiceRoom, LeaveVoiceRoom, ListDirectMessageVoiceRooms, ListServerVoiceRooms,
    RealtimeKind, RealtimeModule, ServerRoleKind, ServerRolePermission, ServerVoiceRoomsSnapshot,
    StopVoiceVideoStream, VoiceChatKind, VoiceRoomSnapshot, VoiceVideoStreamEnded,
};
use cheenhub_contracts::rest::{AuthUser, ServerRoomKind};
use chrono::Utc;
use uuid::Uuid;

use crate::features::social::{self, DirectMessageVoiceAccess, SocialError};
use crate::features::voice_chat::infrastructure::VoicePresence;
use crate::state::AppState;

mod avatar;
mod fanout;
mod presence;

pub(crate) use avatar::update_user_avatar;
use fanout::{
    direct_message_voice_target, fanout_removed_rooms, fanout_snapshot, participant_summary,
    room_snapshot, server_voice_target,
};
use presence::active_presence_for_user;

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
    let target = server_voice_target(server_id, room_id);
    let removed = state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id,
            session_id,
            target_kind: target.kind,
            server_id,
            room_id,
            user_id: *user_id,
            nickname: user.nickname.clone(),
            avatar_url: user.avatar_url.clone(),
            joined_at: Utc::now(),
        })
        .await;

    fanout_removed_rooms(state, removed, Some(target)).await;
    let snapshot = room_snapshot(state, target).await;
    fanout_snapshot(state, target, snapshot.clone()).await;

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
    let target = server_voice_target(server_id, room_id);
    let removed = state
        .voice_presence_store
        .leave_room(&realtime_stream_id, target.kind, &server_id, &room_id)
        .await;

    if removed.is_empty() {
        ensure_room_voice_available(state, user_id, &server_id, &room_id).await?;
        return Ok(room_snapshot(state, target).await);
    }

    let snapshot = room_snapshot(state, target).await;
    fanout_snapshot(state, target, snapshot.clone()).await;

    Ok(snapshot)
}

/// Входит в голосовой звонок личного диалога и возвращает текущий снимок участников.
pub(crate) async fn join_direct_message_room(
    state: &AppState,
    realtime_stream_id: Uuid,
    session_id: Uuid,
    user: &AuthUser,
    user_id: &Uuid,
    request: JoinDirectMessageVoiceRoom,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let conversation_id = parse_id(&request.conversation_id, "Диалог не найден.")?;
    let access = ensure_direct_message_voice_available(state, user_id, &conversation_id).await?;
    let target = direct_message_voice_target(access.conversation_id);
    let removed = state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id,
            session_id,
            target_kind: target.kind,
            server_id: target.server_id,
            room_id: target.room_id,
            user_id: *user_id,
            nickname: user.nickname.clone(),
            avatar_url: user.avatar_url.clone(),
            joined_at: Utc::now(),
        })
        .await;

    tracing::info!(
        conversation_id = %access.conversation_id,
        user_id = %user_id,
        "joined direct message voice room"
    );
    fanout_removed_rooms(state, removed, Some(target)).await;
    let snapshot = room_snapshot(state, target).await;
    fanout_snapshot(state, target, snapshot.clone()).await;

    Ok(snapshot)
}

/// Покидает голосовой звонок личного диалога и возвращает текущий снимок участников.
pub(crate) async fn leave_direct_message_room(
    state: &AppState,
    realtime_stream_id: Uuid,
    user_id: &Uuid,
    request: LeaveDirectMessageVoiceRoom,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let conversation_id = parse_id(&request.conversation_id, "Диалог не найден.")?;
    let target = direct_message_voice_target(conversation_id);
    let removed = state
        .voice_presence_store
        .leave_room(
            &realtime_stream_id,
            target.kind,
            &target.server_id,
            &target.room_id,
        )
        .await;

    if removed.is_empty() {
        ensure_direct_message_voice_available(state, user_id, &conversation_id).await?;
        return Ok(room_snapshot(state, target).await);
    }

    tracing::info!(
        conversation_id = %conversation_id,
        user_id = %user_id,
        "left direct message voice room"
    );
    let snapshot = room_snapshot(state, target).await;
    fanout_snapshot(state, target, snapshot.clone()).await;

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

    let target = server_voice_target(server_id, room_id);
    let snapshot = room_snapshot(state, target).await;
    fanout_snapshot(state, target, snapshot.clone()).await;

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

/// Перечисляет активные голосовые звонки личных диалогов пользователя.
pub(crate) async fn list_direct_message_voice_rooms(
    state: &AppState,
    user_id: &Uuid,
    _request: ListDirectMessageVoiceRooms,
) -> Result<DirectMessageVoiceRoomsSnapshot, VoiceChatApplicationError> {
    let conversations = social::direct_message_voice_accesses_for_user(state, user_id)
        .await
        .map_err(social_error)?;
    let mut rooms = Vec::new();
    for conversation in conversations {
        let target = direct_message_voice_target(conversation.conversation_id);
        let snapshot = room_snapshot(state, target).await;
        if !snapshot.participants.is_empty() {
            rooms.push(snapshot);
        }
    }

    tracing::debug!(
        user_id = %user_id,
        active_direct_message_voice_rooms = rooms.len(),
        "listed direct message voice room participants"
    );

    Ok(DirectMessageVoiceRoomsSnapshot { rooms })
}

/// Рассылает участникам комнаты событие остановки видеопотока отправителя.
pub(crate) async fn stop_video_stream(
    state: &AppState,
    realtime_stream_id: Uuid,
    session_id: Uuid,
    user_id: &Uuid,
    request: StopVoiceVideoStream,
) -> Result<(), VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let Some(presence) = active_presence_for_user(state, &room_id, user_id).await else {
        return Err(VoiceChatApplicationError::NotFound(
            "Пользователь не находится в этой голосовой комнате.".to_owned(),
        ));
    };

    if presence.server_id != server_id || presence.room_id != room_id {
        return Err(VoiceChatApplicationError::BadRequest(
            "Комната не найдена.".to_owned(),
        ));
    }
    if presence.realtime_stream_id != realtime_stream_id || presence.session_id != session_id {
        return Err(VoiceChatApplicationError::Unauthorized(
            "Видеопоток принадлежит другой realtime-сессии.".to_owned(),
        ));
    }

    let recipients = state
        .voice_presence_store
        .room_participants(presence.target_kind, &server_id, &room_id)
        .await;
    let stream_ids = recipients
        .iter()
        .filter(|recipient| recipient.realtime_stream_id != realtime_stream_id)
        .map(|recipient| recipient.realtime_stream_id)
        .collect::<Vec<_>>();
    tracing::info!(
        server_id = %server_id,
        room_id = %room_id,
        target_kind = ?presence.target_kind,
        user_id = %user_id,
        source = ?request.source,
        recipients = stream_ids.len(),
        "fanning out voice video stream ended event"
    );

    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::VoiceChat,
            &server_id,
            RealtimeKind::VoiceChat(VoiceChatKind::VideoStreamEnded),
            &stream_ids,
            VoiceVideoStreamEnded {
                server_id: server_id.to_string(),
                room_id: room_id.to_string(),
                user_id: user_id.to_string(),
                source: request.source,
            },
        )
        .await;

    Ok(())
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
    for target in rooms {
        let snapshot = room_snapshot(state, target).await;
        fanout_snapshot(state, target, snapshot).await;
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

async fn ensure_direct_message_voice_available(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<DirectMessageVoiceAccess, VoiceChatApplicationError> {
    social::direct_message_voice_access(state, user_id, conversation_id)
        .await
        .map_err(social_error)
}

fn social_error(error: SocialError) -> VoiceChatApplicationError {
    match error {
        SocialError::BadRequest(message) => VoiceChatApplicationError::BadRequest(message),
        SocialError::Unauthorized(message) | SocialError::Conflict(message) => {
            VoiceChatApplicationError::Unauthorized(message)
        }
        SocialError::NotFound(message) => VoiceChatApplicationError::NotFound(message),
        SocialError::Internal(error) => VoiceChatApplicationError::Internal(error),
    }
}

fn parse_id(value: &str, message: &str) -> Result<Uuid, VoiceChatApplicationError> {
    Uuid::parse_str(value).map_err(|_| VoiceChatApplicationError::BadRequest(message.to_owned()))
}

#[cfg(test)]
mod tests;
