//! Realtime-команды и события signaling личного звонка.

use cheenhub_contracts::realtime::{
    CancelDirectCall, DirectCallLifecycleEvent, DirectCallResponse, DirectCallSnapshot,
    DirectCallsSnapshot, EndDirectCall, ListDirectCalls, RealtimeEnvelope, RealtimeKind,
    RealtimeModule, RespondDirectCall, StartDirectCall, VoiceChatKind,
};
use dioxus::logger::tracing::warn;
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Начинает личный звонок и возвращает его актуальное состояние.
pub(super) async fn start(
    realtime: &RealtimeHandle,
    conversation_id: String,
) -> Result<DirectCallSnapshot, RealtimeError> {
    let event: DirectCallLifecycleEvent = realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::StartDirectCall),
            StartDirectCall { conversation_id },
        )
        .await?;
    Ok(event.call)
}

/// Отправляет решение по входящему личному звонку.
pub(super) async fn respond(
    realtime: &RealtimeHandle,
    call_id: String,
    response: DirectCallResponse,
) -> Result<DirectCallSnapshot, RealtimeError> {
    let event: DirectCallLifecycleEvent = realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::RespondDirectCall),
            RespondDirectCall { call_id, response },
        )
        .await?;
    Ok(event.call)
}

/// Отменяет исходящий личный звонок до ответа.
pub(super) async fn cancel(
    realtime: &RealtimeHandle,
    call_id: String,
) -> Result<DirectCallSnapshot, RealtimeError> {
    let event: DirectCallLifecycleEvent = realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::CancelDirectCall),
            CancelDirectCall { call_id },
        )
        .await?;
    Ok(event.call)
}

/// Завершает принятый личный звонок.
pub(super) async fn end(
    realtime: &RealtimeHandle,
    call_id: String,
) -> Result<DirectCallSnapshot, RealtimeError> {
    let event: DirectCallLifecycleEvent = realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::EndDirectCall),
            EndDirectCall { call_id },
        )
        .await?;
    Ok(event.call)
}

/// Загружает незавершённые личные звонки текущего пользователя.
pub(super) async fn list(realtime: &RealtimeHandle) -> Result<DirectCallsSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::ListDirectCalls),
            ListDirectCalls,
        )
        .await
}

/// Подписывается на адресные изменения lifecycle личных звонков.
pub(super) fn subscribe(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<DirectCallLifecycleEvent> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(event) = decode_lifecycle(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    });

    receiver
}

fn decode_lifecycle(envelope: RealtimeEnvelope) -> Option<DirectCallLifecycleEvent> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::DirectCallLifecycleEvent)
    {
        return None;
    }

    match serde_json::from_value(envelope.payload) {
        Ok(event) => Some(event),
        Err(error) => {
            warn!(%error, "failed to decode direct call lifecycle event");
            None
        }
    }
}
