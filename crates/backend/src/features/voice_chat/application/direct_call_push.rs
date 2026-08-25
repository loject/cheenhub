//! Доставка короткоживущих push-событий lifecycle личного звонка.

use cheenhub_contracts::realtime::DirectCallEndReason;
use chrono::{DateTime, Utc};

use crate::features::push_notifications::{CallEndedPush, IncomingCallPush};
use crate::features::voice_chat::infrastructure::DirectCall;
use crate::state::AppState;

pub(super) async fn enqueue_incoming_call_push(state: &AppState, call: &DirectCall) {
    if !call.callee_notified {
        return;
    }
    let payload = IncomingCallPush::new(
        call.id,
        call.conversation_id,
        call.caller_user_id,
        &call.caller_nickname,
        call.caller_avatar_url.as_deref(),
        call.started_at,
        call.expires_at,
    );
    match state
        .push_notifications
        .enqueue_incoming_call(call.callee_user_id, payload)
        .await
    {
        Ok(installations) => tracing::debug!(
            call_id = %call.id,
            callee_user_id = %call.callee_user_id,
            installations,
            "queued incoming-call push"
        ),
        Err(error) => tracing::warn!(
            call_id = %call.id,
            callee_user_id = %call.callee_user_id,
            %error,
            "failed to queue incoming-call push"
        ),
    }
}

pub(super) async fn enqueue_call_ended_push(
    state: &AppState,
    call: &DirectCall,
    reason: DirectCallEndReason,
    ended_at: DateTime<Utc>,
) {
    if !call.callee_notified {
        return;
    }
    let reason = match reason {
        DirectCallEndReason::Cancelled => "cancelled",
        DirectCallEndReason::Declined => "declined",
        DirectCallEndReason::TimedOut => "timed_out",
        DirectCallEndReason::Ended => "ended",
    };
    let payload = CallEndedPush::new(call.id, reason, ended_at);
    match state
        .push_notifications
        .enqueue_call_ended(call.callee_user_id, payload)
        .await
    {
        Ok(installations) => tracing::debug!(
            call_id = %call.id,
            callee_user_id = %call.callee_user_id,
            installations,
            "queued call-ended push"
        ),
        Err(error) => tracing::warn!(
            call_id = %call.id,
            callee_user_id = %call.callee_user_id,
            %error,
            "failed to queue call-ended push"
        ),
    }
}
