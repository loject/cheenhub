//! Voice chat realtime helpers.

use bytes::Bytes;
use cheenhub_contracts::media::{
    MEDIA_DATAGRAM_FLAG_KEY_FRAME, MediaCodec, MediaDatagram, MediaDatagramKind,
};
use cheenhub_contracts::realtime::{
    JoinVoiceRoom, KickVoiceMember, LeaveVoiceRoom, ListServerVoiceRooms, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, ServerVoiceRoomsSnapshot, VoiceChatKind, VoiceRoomSnapshot,
};
use futures_channel::mpsc;
use futures_util::StreamExt;
use uuid::Uuid;

use crate::features::microphone::{EncodedMicrophoneFrame, MicrophoneCodec};
use crate::features::realtime::{RealtimeError, RealtimeHandle};
use crate::features::screen_share::{EncodedScreenShareFrame, ScreenShareCodec};

use super::screen_fragments;

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

/// Inbound relayed screen sharing frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InboundScreenFrame {
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
    /// Raw encoded VP9 frame bytes.
    pub(crate) bytes: Vec<u8>,
    /// Whether this frame can start a decoder stream.
    pub(crate) key_frame: bool,
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

/// Loads active voice room participant snapshots for one server.
pub(crate) async fn list_server_voice_rooms(
    realtime: &RealtimeHandle,
    server_id: String,
) -> Result<ServerVoiceRoomsSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::ListServerVoiceRooms),
            ListServerVoiceRooms { server_id },
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

/// Subscribes to inbound relayed screen sharing frames for this tab.
pub(crate) fn subscribe_screen_frames(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<InboundScreenFrame> {
    let datagrams = realtime.subscribe_datagrams();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut datagrams = datagrams;
        let mut reassembler = screen_fragments::ScreenFrameReassembler::default();
        while let Some(bytes) = datagrams.next().await {
            let Some(datagram) = decode_screen_datagram(&bytes) else {
                continue;
            };
            let frame = if screen_fragments::is_fragmented(&datagram) {
                reassembler.push(datagram).map(screen_frame_from_datagram)
            } else {
                Some(screen_frame_from_datagram(datagram))
            };
            let Some(frame) = frame else {
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
        flags: 0,
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

/// Sends one encoded screen sharing frame to the active voice room.
pub(crate) async fn send_screen_frame(
    realtime: &RealtimeHandle,
    _server_id: &str,
    room_id: &str,
    frame: EncodedScreenShareFrame,
) -> Result<(), RealtimeError> {
    let room_id =
        Uuid::parse_str(room_id).map_err(|_| RealtimeError::new("Voice room id is invalid."))?;
    let codec = match frame.codec {
        ScreenShareCodec::Vp9 => MediaCodec::Vp9,
    };
    let (_width, _height) = (frame.width, frame.height);
    let flags = if frame.key_frame {
        MEDIA_DATAGRAM_FLAG_KEY_FRAME
    } else {
        0
    };
    for datagram in screen_fragments::screen_frame_datagrams(room_id, codec, flags, frame)? {
        let bytes = datagram.encode().map_err(|error| {
            RealtimeError::new(format!("Failed to encode screen frame: {error}"))
        })?;
        realtime.send_unreliable_bytes(Bytes::from(bytes)).await?;
    }

    Ok(())
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

fn decode_screen_datagram(bytes: &[u8]) -> Option<MediaDatagram> {
    let datagram = MediaDatagram::decode(bytes).ok()?;
    if datagram.kind != MediaDatagramKind::ScreenFrame || datagram.codec != MediaCodec::Vp9 {
        return None;
    }

    Some(datagram)
}

fn screen_frame_from_datagram(datagram: MediaDatagram) -> InboundScreenFrame {
    InboundScreenFrame {
        room_id: datagram.room_id.to_string(),
        sender_user_id: datagram.sender_user_id.to_string(),
        sequence: datagram.sequence,
        timestamp_us: datagram.timestamp_us,
        duration_us: datagram.duration_us,
        bytes: datagram.payload,
        key_frame: datagram.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0,
    }
}
