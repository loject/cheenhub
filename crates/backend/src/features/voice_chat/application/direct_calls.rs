//! Сценарии приглашений и lifecycle личных звонков.

use std::time::Duration as StdDuration;

use cheenhub_contracts::realtime::{
    CancelDirectCall, DirectCallEndReason, DirectCallLifecycleEvent, DirectCallResponse,
    DirectCallSnapshot, DirectCallState, DirectCallsSnapshot, EndDirectCall, ListDirectCalls,
    RealtimeKind, RealtimeModule, RespondDirectCall, StartDirectCall, VoiceChatKind,
};
use cheenhub_contracts::rest::AuthUser;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::features::auth::application::auth_user;
use crate::features::social;
use crate::features::voice_chat::infrastructure::{
    DirectCall, DirectCallStoreError, DirectCallTransition,
};
use crate::state::AppState;

use super::direct_call_push::{enqueue_call_ended_push, enqueue_incoming_call_push};

use super::{
    VoiceChatApplicationError, ensure_direct_message_voice_available, parse_id, social_error,
};

const DIRECT_CALL_RING_TIMEOUT: Duration = Duration::seconds(45);

/// Создаёт приглашение в личный звонок и уведомляет вызываемого пользователя.
pub(crate) async fn start_direct_call(
    state: &AppState,
    caller: &AuthUser,
    caller_user_id: &Uuid,
    request: StartDirectCall,
) -> Result<DirectCallLifecycleEvent, VoiceChatApplicationError> {
    start_direct_call_at(
        state,
        caller,
        caller_user_id,
        request,
        Utc::now(),
        DIRECT_CALL_RING_TIMEOUT,
        true,
    )
    .await
}

async fn start_direct_call_at(
    state: &AppState,
    caller: &AuthUser,
    caller_user_id: &Uuid,
    request: StartDirectCall,
    now: DateTime<Utc>,
    ring_timeout: Duration,
    spawn_expiry: bool,
) -> Result<DirectCallLifecycleEvent, VoiceChatApplicationError> {
    expire_pending_calls(state, now).await;
    let conversation_id = parse_id(&request.conversation_id, "Диалог не найден.")?;
    ensure_direct_message_voice_available(state, caller_user_id, &conversation_id).await?;
    let participants = social::direct_message_voice_user_ids(state, &conversation_id)
        .await
        .map_err(social_error)?;
    let callee_user_id = participants
        .into_iter()
        .find(|participant| participant != caller_user_id)
        .ok_or_else(|| VoiceChatApplicationError::NotFound("Диалог не найден.".to_owned()))?;
    let callee_account = state
        .auth_store
        .find_user_by_id(&callee_user_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
        .ok_or_else(|| VoiceChatApplicationError::NotFound("Пользователь не найден.".to_owned()))?;
    let callee = auth_user(state, &callee_account);
    let call = state
        .direct_call_store
        .start(DirectCall {
            id: Uuid::new_v4(),
            conversation_id,
            caller_user_id: *caller_user_id,
            caller_nickname: caller.nickname.clone(),
            caller_avatar_url: caller.avatar_url.clone(),
            callee_user_id,
            callee_nickname: callee.nickname,
            callee_avatar_url: callee.avatar_url,
            started_at: now,
            expires_at: now + ring_timeout,
            answered_at: None,
            callee_notified: true,
        })
        .await
        .map_err(|error| {
            tracing::warn!(
                conversation_id = %conversation_id,
                caller_user_id = %caller_user_id,
                callee_user_id = %callee_user_id,
                error_kind = store_error_kind(&error),
                "rejected direct call start"
            );
            map_store_error(error)
        })?;

    tracing::info!(
        call_id = %call.id,
        conversation_id = %call.conversation_id,
        caller_user_id = %call.caller_user_id,
        callee_user_id = %call.callee_user_id,
        callee_notified = call.callee_notified,
        expires_at = %call.expires_at,
        "started direct call"
    );
    fanout_call_event(state, &call, None, None).await;
    enqueue_incoming_call_push(state, &call).await;
    if spawn_expiry {
        spawn_call_expiry(state.clone(), call.id, call.expires_at);
    }

    Ok(lifecycle_event(call.caller_user_id, &call, None, None))
}

/// Принимает или отклоняет входящий личный звонок.
pub(crate) async fn respond_direct_call(
    state: &AppState,
    callee_user_id: &Uuid,
    request: RespondDirectCall,
) -> Result<DirectCallLifecycleEvent, VoiceChatApplicationError> {
    let call_id = parse_id(&request.call_id, "Звонок не найден.")?;
    let now = Utc::now();
    let transition = match state
        .direct_call_store
        .respond(
            &call_id,
            callee_user_id,
            request.response == DirectCallResponse::Accept,
            now,
        )
        .await
    {
        Ok(transition) => transition,
        Err(DirectCallStoreError::Expired(call)) => {
            notify_timed_out(state, &call, now).await;
            return Err(VoiceChatApplicationError::NotFound(
                "Время ожидания ответа истекло.".to_owned(),
            ));
        }
        Err(error) => {
            tracing::warn!(
                call_id = %call_id,
                user_id = %callee_user_id,
                error_kind = store_error_kind(&error),
                "rejected direct call response"
            );
            return Err(map_store_error(error));
        }
    };
    let (call, end_reason) = match transition {
        DirectCallTransition::Accepted(call) => {
            tracing::info!(
                call_id = %call.id,
                conversation_id = %call.conversation_id,
                callee_user_id = %callee_user_id,
                "accepted direct call"
            );
            (call, None)
        }
        DirectCallTransition::Ended(call) => {
            tracing::info!(
                call_id = %call.id,
                conversation_id = %call.conversation_id,
                callee_user_id = %callee_user_id,
                "declined direct call"
            );
            (call, Some(DirectCallEndReason::Declined))
        }
    };
    fanout_call_event(state, &call, end_reason, end_reason.map(|_| now)).await;

    Ok(lifecycle_event(
        *callee_user_id,
        &call,
        end_reason,
        end_reason.map(|_| now),
    ))
}

/// Отменяет ожидающий личный звонок от имени инициатора.
pub(crate) async fn cancel_direct_call(
    state: &AppState,
    caller_user_id: &Uuid,
    request: CancelDirectCall,
) -> Result<DirectCallLifecycleEvent, VoiceChatApplicationError> {
    let call_id = parse_id(&request.call_id, "Звонок не найден.")?;
    let now = Utc::now();
    let call = match state
        .direct_call_store
        .cancel(&call_id, caller_user_id, now)
        .await
    {
        Ok(call) => call,
        Err(DirectCallStoreError::Expired(call)) => {
            notify_timed_out(state, &call, now).await;
            return Err(VoiceChatApplicationError::NotFound(
                "Время ожидания ответа истекло.".to_owned(),
            ));
        }
        Err(error) => {
            tracing::warn!(
                call_id = %call_id,
                user_id = %caller_user_id,
                error_kind = store_error_kind(&error),
                "rejected direct call cancellation"
            );
            return Err(map_store_error(error));
        }
    };
    tracing::info!(
        call_id = %call.id,
        conversation_id = %call.conversation_id,
        caller_user_id = %caller_user_id,
        "cancelled direct call"
    );
    fanout_call_event(
        state,
        &call,
        Some(DirectCallEndReason::Cancelled),
        Some(now),
    )
    .await;

    Ok(lifecycle_event(
        *caller_user_id,
        &call,
        Some(DirectCallEndReason::Cancelled),
        Some(now),
    ))
}

/// Завершает принятый личный звонок от имени одного из участников.
pub(crate) async fn end_direct_call(
    state: &AppState,
    user_id: &Uuid,
    request: EndDirectCall,
) -> Result<DirectCallLifecycleEvent, VoiceChatApplicationError> {
    let call_id = parse_id(&request.call_id, "Звонок не найден.")?;
    let call = state
        .direct_call_store
        .end(&call_id, user_id)
        .await
        .map_err(|error| {
            tracing::warn!(
                call_id = %call_id,
                user_id = %user_id,
                error_kind = store_error_kind(&error),
                "rejected direct call end"
            );
            map_store_error(error)
        })?;
    let now = Utc::now();
    tracing::info!(
        call_id = %call.id,
        conversation_id = %call.conversation_id,
        user_id = %user_id,
        "ended direct call"
    );
    fanout_call_event(state, &call, Some(DirectCallEndReason::Ended), Some(now)).await;

    Ok(lifecycle_event(
        *user_id,
        &call,
        Some(DirectCallEndReason::Ended),
        Some(now),
    ))
}

/// Возвращает актуальные ожидающие и принятые личные звонки пользователя.
pub(crate) async fn list_direct_calls(
    state: &AppState,
    user_id: &Uuid,
    _request: ListDirectCalls,
) -> Result<DirectCallsSnapshot, VoiceChatApplicationError> {
    expire_pending_calls(state, Utc::now()).await;
    let calls = state
        .direct_call_store
        .list_for_user(user_id)
        .await
        .iter()
        .map(|call| snapshot(call, None, None))
        .collect::<Vec<_>>();
    tracing::debug!(
        user_id = %user_id,
        direct_calls = calls.len(),
        "listed direct calls"
    );
    Ok(DirectCallsSnapshot { calls })
}

/// Завершает активный lifecycle после выхода участника из media presence личного диалога.
pub(super) async fn end_direct_call_for_presence(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: &Uuid,
) {
    let Some(call) = state
        .direct_call_store
        .end_active_for_conversation(conversation_id, user_id)
        .await
    else {
        return;
    };
    let now = Utc::now();
    tracing::info!(
        call_id = %call.id,
        conversation_id = %call.conversation_id,
        user_id = %user_id,
        "ended direct call after media presence left"
    );
    fanout_call_event(state, &call, Some(DirectCallEndReason::Ended), Some(now)).await;
}

fn spawn_call_expiry(state: AppState, call_id: Uuid, expires_at: DateTime<Utc>) {
    tokio::spawn(async move {
        let delay = (expires_at - Utc::now())
            .to_std()
            .unwrap_or(StdDuration::ZERO);
        tokio::time::sleep(delay).await;
        if let Some(call) = state.direct_call_store.expire(&call_id, Utc::now()).await {
            notify_timed_out(&state, &call, Utc::now()).await;
        }
    });
}

async fn expire_pending_calls(state: &AppState, now: DateTime<Utc>) {
    for call in state.direct_call_store.expire_pending(now).await {
        notify_timed_out(state, &call, now).await;
    }
}

async fn notify_timed_out(state: &AppState, call: &DirectCall, ended_at: DateTime<Utc>) {
    tracing::info!(
        call_id = %call.id,
        conversation_id = %call.conversation_id,
        caller_user_id = %call.caller_user_id,
        callee_user_id = %call.callee_user_id,
        "direct call timed out"
    );
    fanout_call_event(
        state,
        call,
        Some(DirectCallEndReason::TimedOut),
        Some(ended_at),
    )
    .await;
}

async fn fanout_call_event(
    state: &AppState,
    call: &DirectCall,
    end_reason: Option<DirectCallEndReason>,
    ended_at: Option<DateTime<Utc>>,
) {
    if let (Some(reason), Some(ended_at)) = (end_reason, ended_at) {
        enqueue_call_ended_push(state, call, reason, ended_at).await;
    }

    let mut user_ids = vec![call.caller_user_id];
    if call.callee_notified {
        user_ids.push(call.callee_user_id);
    }
    for user_id in user_ids {
        let recipient_stream_count = state
            .realtime_hub
            .fanout_to_user_streams(
                RealtimeModule::VoiceChat,
                RealtimeKind::VoiceChat(VoiceChatKind::DirectCallLifecycleEvent),
                &[user_id],
                lifecycle_event(user_id, call, end_reason, ended_at),
            )
            .await;
        tracing::debug!(
            call_id = %call.id,
            recipient_user_id = %user_id,
            recipient_stream_count,
            "fanned out direct call lifecycle event"
        );
    }
}

fn lifecycle_event(
    recipient_user_id: Uuid,
    call: &DirectCall,
    end_reason: Option<DirectCallEndReason>,
    ended_at: Option<DateTime<Utc>>,
) -> DirectCallLifecycleEvent {
    DirectCallLifecycleEvent {
        recipient_user_id: recipient_user_id.to_string(),
        call: snapshot(call, end_reason, ended_at),
    }
}

fn snapshot(
    call: &DirectCall,
    end_reason: Option<DirectCallEndReason>,
    ended_at: Option<DateTime<Utc>>,
) -> DirectCallSnapshot {
    DirectCallSnapshot {
        call_id: call.id.to_string(),
        conversation_id: call.conversation_id.to_string(),
        caller_user_id: call.caller_user_id.to_string(),
        caller_nickname: call.caller_nickname.clone(),
        caller_avatar_url: call.caller_avatar_url.clone(),
        callee_user_id: call.callee_user_id.to_string(),
        callee_nickname: call.callee_nickname.clone(),
        callee_avatar_url: call.callee_avatar_url.clone(),
        state: if end_reason.is_some() {
            DirectCallState::Ended
        } else if call.answered_at.is_some() {
            DirectCallState::Active
        } else {
            DirectCallState::Ringing
        },
        started_at: call.started_at.to_rfc3339(),
        answered_at: call.answered_at.map(|value| value.to_rfc3339()),
        ended_at: ended_at.map(|value| value.to_rfc3339()),
        end_reason,
    }
}

fn map_store_error(error: DirectCallStoreError) -> VoiceChatApplicationError {
    match error {
        DirectCallStoreError::CallerBusy => VoiceChatApplicationError::BadRequest(
            "У тебя уже есть незавершённый личный звонок.".to_owned(),
        ),
        DirectCallStoreError::NotFound => {
            VoiceChatApplicationError::NotFound("Звонок не найден.".to_owned())
        }
        DirectCallStoreError::Unauthorized => {
            VoiceChatApplicationError::Unauthorized("Нет доступа к звонку.".to_owned())
        }
        DirectCallStoreError::InvalidState => VoiceChatApplicationError::BadRequest(
            "Действие недоступно в текущем состоянии звонка.".to_owned(),
        ),
        DirectCallStoreError::Expired(_) => {
            VoiceChatApplicationError::NotFound("Время ожидания ответа истекло.".to_owned())
        }
    }
}

fn store_error_kind(error: &DirectCallStoreError) -> &'static str {
    match error {
        DirectCallStoreError::CallerBusy => "caller_busy",
        DirectCallStoreError::NotFound => "not_found",
        DirectCallStoreError::Unauthorized => "unauthorized",
        DirectCallStoreError::InvalidState => "invalid_state",
        DirectCallStoreError::Expired(_) => "expired",
    }
}

#[cfg(test)]
pub(super) async fn start_direct_call_without_expiry_task(
    state: &AppState,
    caller: &AuthUser,
    caller_user_id: &Uuid,
    request: StartDirectCall,
    now: DateTime<Utc>,
    ring_timeout: Duration,
) -> Result<DirectCallLifecycleEvent, VoiceChatApplicationError> {
    start_direct_call_at(
        state,
        caller,
        caller_user_id,
        request,
        now,
        ring_timeout,
        false,
    )
    .await
}

#[cfg(test)]
pub(super) async fn expire_direct_calls_at(state: &AppState, now: DateTime<Utc>) {
    expire_pending_calls(state, now).await;
}
