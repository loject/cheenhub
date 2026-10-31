//! Realtime-команды и адресные события приглашений в голосовые комнаты сервера.

use cheenhub_contracts::realtime::{
    CancelVoiceRoomInvite, ListVoiceRoomInviteCandidates, ListVoiceRoomInvites, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, RespondVoiceRoomInvite, SendVoiceRoomInvite, VoiceChatKind,
    VoiceRoomInviteCandidatesSnapshot, VoiceRoomInviteLifecycleEvent, VoiceRoomInviteResponse,
    VoiceRoomInvitesSnapshot,
};
use dioxus::logger::tracing::warn;
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Загружает участников сервера, которым можно отправить приглашение.
pub(super) async fn candidates(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<VoiceRoomInviteCandidatesSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::ListVoiceRoomInviteCandidates),
            ListVoiceRoomInviteCandidates { server_id, room_id },
        )
        .await
}

/// Отправляет приглашение и возвращает его текущее состояние.
pub(super) async fn send(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    recipient_user_id: String,
) -> Result<VoiceRoomInviteLifecycleEvent, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::SendVoiceRoomInvite),
            SendVoiceRoomInvite {
                server_id,
                room_id,
                recipient_user_id,
            },
        )
        .await
}

/// Отправляет принятие или отклонение без автоматического подключения медиа.
pub(super) async fn respond(
    realtime: &RealtimeHandle,
    invite_id: String,
    response: VoiceRoomInviteResponse,
) -> Result<VoiceRoomInviteLifecycleEvent, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::RespondVoiceRoomInvite),
            RespondVoiceRoomInvite {
                invite_id,
                response,
            },
        )
        .await
}

/// Отменяет отправленное приглашение.
pub(super) async fn cancel(
    realtime: &RealtimeHandle,
    invite_id: String,
) -> Result<VoiceRoomInviteLifecycleEvent, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::CancelVoiceRoomInvite),
            CancelVoiceRoomInvite { invite_id },
        )
        .await
}

/// Восстанавливает приглашения после переподключения realtime.
pub(super) async fn list(
    realtime: &RealtimeHandle,
) -> Result<VoiceRoomInvitesSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::ListVoiceRoomInvites),
            ListVoiceRoomInvites,
        )
        .await
}

/// Слушает события переходов приглашений для всех устройств аккаунта.
pub(super) fn subscribe(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<VoiceRoomInviteLifecycleEvent> {
    let mut events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();
    dioxus::prelude::spawn(async move {
        while let Some(envelope) = events.next().await {
            let Some(event) = decode_event(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    });
    receiver
}

fn decode_event(envelope: RealtimeEnvelope) -> Option<VoiceRoomInviteLifecycleEvent> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomInviteLifecycleEvent)
    {
        return None;
    }
    match serde_json::from_value(envelope.payload) {
        Ok(event) => Some(event),
        Err(error) => {
            warn!(%error, "failed to decode voice room invite lifecycle event");
            None
        }
    }
}
