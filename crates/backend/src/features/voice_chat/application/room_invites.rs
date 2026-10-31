//! Приглашения участников сервера в активную голосовую комнату.

use std::collections::HashSet;

use cheenhub_contracts::realtime::{
    CancelVoiceRoomInvite, ListVoiceRoomInviteCandidates, ListVoiceRoomInvites, RealtimeKind,
    RealtimeModule, RespondVoiceRoomInvite, SendVoiceRoomInvite, VoiceChatKind,
    VoiceRoomInviteCandidate, VoiceRoomInviteCandidatesSnapshot, VoiceRoomInviteLifecycleEvent,
    VoiceRoomInviteResponse, VoiceRoomInvitesSnapshot,
};
use cheenhub_contracts::rest::AuthUser;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::features::auth::application::auth_user;
use crate::features::voice_chat::infrastructure::{
    VoicePresenceTargetKind, VoiceRoomInvite, VoiceRoomInviteStoreError,
};
use crate::state::AppState;

use super::{VoiceChatApplicationError, ensure_room_voice_available, parse_id};

const ROOM_INVITE_TIMEOUT: Duration = Duration::seconds(45);

/// Возвращает доступных для приглашения участников того же сервера.
pub(crate) async fn list_voice_room_invite_candidates(
    state: &AppState,
    realtime_stream_id: Uuid,
    caller_user_id: &Uuid,
    request: ListVoiceRoomInviteCandidates,
) -> Result<VoiceRoomInviteCandidatesSnapshot, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    ensure_sender_in_room(
        state,
        realtime_stream_id,
        caller_user_id,
        server_id,
        room_id,
    )
    .await?;

    let server = state
        .server_store
        .find_server(&server_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
        .ok_or_else(|| VoiceChatApplicationError::NotFound("Сервер не найден.".to_owned()))?;
    let mut members: HashSet<Uuid> = state
        .server_store
        .list_active_server_members(&server_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
        .into_iter()
        .map(|member| member.user_id)
        .collect();
    members.insert(server.owner_user_id);
    members.remove(caller_user_id);

    let present = state
        .voice_presence_store
        .room_participants(VoicePresenceTargetKind::Server, &server_id, &room_id)
        .await;
    for participant in present {
        members.remove(&participant.user_id);
    }

    let mut candidates = Vec::with_capacity(members.len());
    for member_id in members {
        let Some(account) = state
            .auth_store
            .find_user_by_id(&member_id)
            .await
            .map_err(VoiceChatApplicationError::Internal)?
        else {
            continue;
        };
        let member = auth_user(state, &account);
        candidates.push(VoiceRoomInviteCandidate {
            user_id: member_id.to_string(),
            nickname: member.nickname,
            avatar_url: member.avatar_url,
        });
    }
    candidates.sort_by(|a, b| a.nickname.cmp(&b.nickname).then(a.user_id.cmp(&b.user_id)));
    Ok(VoiceRoomInviteCandidatesSnapshot {
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        candidates,
    })
}

/// Создаёт приглашение только от пользователя, находящегося в комнате в этом realtime-потоке.
pub(crate) async fn send_voice_room_invite(
    state: &AppState,
    realtime_stream_id: Uuid,
    caller: &AuthUser,
    caller_user_id: &Uuid,
    request: SendVoiceRoomInvite,
) -> Result<VoiceRoomInviteLifecycleEvent, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let recipient_id = parse_id(&request.recipient_user_id, "Пользователь не найден.")?;
    ensure_sender_in_room(
        state,
        realtime_stream_id,
        caller_user_id,
        server_id,
        room_id,
    )
    .await?;
    if recipient_id == *caller_user_id {
        return Err(VoiceChatApplicationError::BadRequest(
            "Нельзя пригласить самого себя.".to_owned(),
        ));
    }
    ensure_room_voice_available(state, &recipient_id, &server_id, &room_id).await?;

    let present = state
        .voice_presence_store
        .room_participants(VoicePresenceTargetKind::Server, &server_id, &room_id)
        .await;
    if present
        .iter()
        .any(|participant| participant.user_id == recipient_id)
    {
        return Err(VoiceChatApplicationError::BadRequest(
            "Пользователь уже находится в голосовой комнате.".to_owned(),
        ));
    }
    let server = state
        .server_store
        .find_server(&server_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
        .ok_or_else(|| VoiceChatApplicationError::NotFound("Сервер не найден.".to_owned()))?;
    let room = state
        .server_store
        .find_server_room(&server_id, &room_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
        .ok_or_else(|| VoiceChatApplicationError::NotFound("Комната не найдена.".to_owned()))?;
    let now = Utc::now();
    let invite = state
        .voice_room_invite_store
        .start(
            VoiceRoomInvite {
                id: Uuid::new_v4(),
                server_id,
                server_name: server.name,
                room_id,
                room_name: room.name,
                caller_user_id: *caller_user_id,
                caller_nickname: caller.nickname.clone(),
                caller_avatar_url: caller.avatar_url.clone(),
                recipient_user_id: recipient_id,
                status: cheenhub_contracts::realtime::VoiceRoomInviteStatus::Pending,
                created_at: now,
                expires_at: now + ROOM_INVITE_TIMEOUT,
                resolved_at: None,
            },
            now,
        )
        .await
        .map_err(map_store_error)?;
    fanout_invite_event(state, &invite).await;
    spawn_invite_expiry(state.clone(), invite.id, invite.expires_at);
    Ok(lifecycle_event(&invite))
}

/// Обрабатывает принятие или отклонение приглашения получателем.
pub(crate) async fn respond_voice_room_invite(
    state: &AppState,
    recipient_user_id: &Uuid,
    request: RespondVoiceRoomInvite,
) -> Result<VoiceRoomInviteLifecycleEvent, VoiceChatApplicationError> {
    let invite_id = parse_id(&request.invite_id, "Приглашение не найдено.")?;
    let invite = state
        .voice_room_invite_store
        .find(&invite_id)
        .await
        .ok_or_else(|| VoiceChatApplicationError::NotFound("Приглашение не найдено.".to_owned()))?;
    if invite.recipient_user_id != *recipient_user_id {
        return Err(VoiceChatApplicationError::Unauthorized(
            "Это приглашение адресовано другому пользователю.".to_owned(),
        ));
    }
    if request.response == VoiceRoomInviteResponse::Accept {
        ensure_room_voice_available(state, recipient_user_id, &invite.server_id, &invite.room_id)
            .await?;
    }
    let resolved = match state
        .voice_room_invite_store
        .respond(&invite_id, recipient_user_id, request.response, Utc::now())
        .await
    {
        Ok(resolved) => resolved,
        Err(VoiceRoomInviteStoreError::Expired(expired)) => {
            fanout_invite_event(state, &expired).await;
            return Err(VoiceChatApplicationError::NotFound(
                "Время ожидания приглашения истекло.".to_owned(),
            ));
        }
        Err(error) => return Err(map_store_error(error)),
    };
    fanout_invite_event(state, &resolved).await;
    Ok(lifecycle_event(&resolved))
}

/// Отменяет ожидающее приглашение от имени отправителя.
pub(crate) async fn cancel_voice_room_invite(
    state: &AppState,
    caller_user_id: &Uuid,
    request: CancelVoiceRoomInvite,
) -> Result<VoiceRoomInviteLifecycleEvent, VoiceChatApplicationError> {
    let invite_id = parse_id(&request.invite_id, "Приглашение не найдено.")?;
    let resolved = match state
        .voice_room_invite_store
        .cancel(&invite_id, caller_user_id, Utc::now())
        .await
    {
        Ok(resolved) => resolved,
        Err(VoiceRoomInviteStoreError::Expired(expired)) => {
            fanout_invite_event(state, &expired).await;
            return Err(VoiceChatApplicationError::NotFound(
                "Время ожидания приглашения истекло.".to_owned(),
            ));
        }
        Err(error) => return Err(map_store_error(error)),
    };
    fanout_invite_event(state, &resolved).await;
    Ok(lifecycle_event(&resolved))
}

/// Восстанавливает ожидающие и недавно завершённые приглашения после переподключения.
pub(crate) async fn list_voice_room_invites(
    state: &AppState,
    user_id: &Uuid,
    _request: ListVoiceRoomInvites,
) -> Result<VoiceRoomInvitesSnapshot, VoiceChatApplicationError> {
    for expired in state
        .voice_room_invite_store
        .expire_pending(Utc::now())
        .await
    {
        fanout_invite_event(state, &expired).await;
    }
    let invites = state
        .voice_room_invite_store
        .list_for_user(user_id)
        .await
        .iter()
        .map(VoiceRoomInvite::snapshot)
        .collect();
    Ok(VoiceRoomInvitesSnapshot { invites })
}

async fn ensure_sender_in_room(
    state: &AppState,
    realtime_stream_id: Uuid,
    caller_user_id: &Uuid,
    server_id: Uuid,
    room_id: Uuid,
) -> Result<(), VoiceChatApplicationError> {
    ensure_room_voice_available(state, caller_user_id, &server_id, &room_id).await?;
    let active = state
        .voice_presence_store
        .presence_for_stream(&realtime_stream_id, caller_user_id)
        .is_some_and(|presence| {
            presence.target_kind == VoicePresenceTargetKind::Server
                && presence.server_id == server_id
                && presence.room_id == room_id
        });
    if !active {
        return Err(VoiceChatApplicationError::Unauthorized(
            "Сначала войди в эту голосовую комнату.".to_owned(),
        ));
    }
    Ok(())
}

fn lifecycle_event(invite: &VoiceRoomInvite) -> VoiceRoomInviteLifecycleEvent {
    VoiceRoomInviteLifecycleEvent {
        invite: invite.snapshot(),
    }
}

async fn fanout_invite_event(state: &AppState, invite: &VoiceRoomInvite) {
    let recipient_count = state
        .realtime_hub
        .fanout_to_user_streams(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomInviteLifecycleEvent),
            &[invite.caller_user_id, invite.recipient_user_id],
            lifecycle_event(invite),
        )
        .await;
    tracing::debug!(invite_id = %invite.id, recipient_count, "fanned out voice room invite");
}

fn spawn_invite_expiry(state: AppState, invite_id: Uuid, expires_at: chrono::DateTime<Utc>) {
    tokio::spawn(async move {
        let remaining = (expires_at - Utc::now()).to_std().unwrap_or_default();
        tokio::time::sleep(remaining).await;
        if let Some(expired) = state
            .voice_room_invite_store
            .expire(&invite_id, Utc::now())
            .await
        {
            fanout_invite_event(&state, &expired).await;
        }
    });
}

fn map_store_error(error: VoiceRoomInviteStoreError) -> VoiceChatApplicationError {
    match error {
        VoiceRoomInviteStoreError::AlreadyPending => VoiceChatApplicationError::BadRequest(
            "Приглашение этому пользователю уже отправлено.".to_owned(),
        ),
        VoiceRoomInviteStoreError::NotFound => {
            VoiceChatApplicationError::NotFound("Приглашение не найдено.".to_owned())
        }
        VoiceRoomInviteStoreError::Unauthorized => VoiceChatApplicationError::Unauthorized(
            "Недостаточно прав для этого приглашения.".to_owned(),
        ),
        VoiceRoomInviteStoreError::InvalidState => {
            VoiceChatApplicationError::BadRequest("Приглашение уже завершено.".to_owned())
        }
        VoiceRoomInviteStoreError::Expired(_) => {
            VoiceChatApplicationError::NotFound("Время ожидания приглашения истекло.".to_owned())
        }
    }
}
