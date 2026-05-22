//! Voice chat realtime helpers.

use bytes::Bytes;
use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramKind};
use cheenhub_contracts::realtime::{
    JoinVoiceRoom, KickVoiceMember, LeaveVoiceRoom, RealtimeEnvelope, RealtimeKind, RealtimeModule,
    VoiceChatKind, VoiceRoomSnapshot,
};
use futures_channel::mpsc;
use futures_util::StreamExt;
use uuid::Uuid;

use crate::features::microphone::{EncodedMicrophoneFrame, MicrophoneCodec};
use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Inbound relayed voice frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InboundVoiceFrame {
    /// Target room identifier.
    pub(crate) room_id: String,
    /// Authenticated sender identifier.
    pub(crate) sender_user_id: String,
    /// Sender-local packet sequence.
    pub(crate) sequence: u64,
    /// Capture or encode timestamp in microseconds.
    pub(crate) timestamp_us: u64,
    /// Frame duration in microseconds.
    pub(crate) duration_us: u32,
    /// Raw encoded frame bytes.
    pub(crate) bytes: Vec<u8>,
}

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

/// Kicks one participant from a voice room.
pub(crate) async fn kick_voice_member(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
    user_id: String,
) -> Result<VoiceRoomSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::KickVoiceMember),
            KickVoiceMember {
                server_id,
                room_id,
                user_id,
            },
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

/// Subscribes to inbound relayed voice frames for this tab.
pub(crate) fn subscribe_voice_frames(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<InboundVoiceFrame> {
    let datagrams = realtime.subscribe_datagrams();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut datagrams = datagrams;
        while let Some(bytes) = datagrams.next().await {
            let Some(frame) = decode_voice_frame(&bytes) else {
                continue;
            };
            if sender.unbounded_send(frame).is_err() {
                break;
            }
        }
    });

    receiver
}

/// Sends one encoded microphone frame to the active voice room.
pub(crate) async fn send_voice_frame(
    realtime: &RealtimeHandle,
    _server_id: &str,
    room_id: &str,
    frame: EncodedMicrophoneFrame,
) -> Result<(), RealtimeError> {
    let room_id =
        Uuid::parse_str(room_id).map_err(|_| RealtimeError::new("Voice room id is invalid."))?;
    let codec = match frame.codec {
        MicrophoneCodec::Opus => MediaCodec::Opus,
    };
    let datagram = MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec,
        sequence: frame.sequence,
        timestamp_us: frame.timestamp_us,
        duration_us: frame.duration_us,
        room_id,
        sender_user_id: Uuid::nil(),
        payload: frame.bytes,
    };
    let bytes = datagram
        .encode()
        .map_err(|error| RealtimeError::new(format!("Failed to encode voice frame: {error}")))?;

    realtime.send_unreliable_bytes(Bytes::from(bytes)).await
}

fn decode_participants_changed(envelope: RealtimeEnvelope) -> Option<VoiceRoomSnapshot> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged)
    {
        return None;
    }

    serde_json::from_value::<VoiceRoomSnapshot>(envelope.payload).ok()
}

fn decode_voice_frame(bytes: &[u8]) -> Option<InboundVoiceFrame> {
    let datagram = MediaDatagram::decode(bytes).ok()?;
    if datagram.kind != MediaDatagramKind::VoiceFrame || datagram.codec != MediaCodec::Opus {
        return None;
    }

    Some(InboundVoiceFrame {
        room_id: datagram.room_id.to_string(),
        sender_user_id: datagram.sender_user_id.to_string(),
        sequence: datagram.sequence,
        timestamp_us: datagram.timestamp_us,
        duration_us: datagram.duration_us,
        bytes: datagram.payload,
    })
}
