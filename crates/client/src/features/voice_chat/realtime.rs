//! Voice chat realtime helpers.

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, LeaveVoiceRoom, RealtimeEnvelope, RealtimeKind, RealtimeModule, VoiceChatKind,
    VoiceRoomSnapshot,
};
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Joins one voice-capable room.
pub(crate) async fn join_room(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<VoiceRoomSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::JoinVoiceRoom),
            JoinVoiceRoom { server_id, room_id },
        )
        .await
}

/// Leaves one voice-capable room.
pub(crate) async fn leave_room(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<VoiceRoomSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::LeaveVoiceRoom),
            LeaveVoiceRoom { server_id, room_id },
        )
        .await
}

/// Subscribes to inbound voice participant snapshot events for this tab.
pub(crate) fn subscribe_voice_chat(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<VoiceRoomSnapshot> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(snapshot) = decode_participants_changed(envelope) else {
                continue;
            };
            if sender.unbounded_send(snapshot).is_err() {
                break;
            }
        }
    });

    receiver
}

fn decode_participants_changed(envelope: RealtimeEnvelope) -> Option<VoiceRoomSnapshot> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged)
    {
        return None;
    }

    serde_json::from_value::<VoiceRoomSnapshot>(envelope.payload).ok()
}
