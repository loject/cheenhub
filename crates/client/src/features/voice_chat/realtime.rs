//! Voice chat realtime helpers.

use bytes::Bytes;
use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramKind};
use cheenhub_contracts::realtime::{
    DirectMessageVoiceRoomsSnapshot, JoinDirectMessageVoiceRoom, JoinVoiceRoom, KickVoiceMember,
    LeaveDirectMessageVoiceRoom, LeaveVoiceRoom, ListDirectMessageVoiceRooms, ListServerVoiceRooms,
    RealtimeKind, RealtimeModule, ServerVoiceRoomsSnapshot, StopVoiceVideoStream, VoiceChatKind,
    VoiceRoomSnapshot, VoiceVideoStreamSource,
};
use futures_channel::mpsc;
use futures_util::StreamExt;
use uuid::Uuid;

use crate::features::microphone::{EncodedMicrophoneFrame, MicrophoneCodec};
use crate::features::realtime::{RealtimeError, RealtimeHandle};
use crate::features::screen_share::{EncodedScreenShareFrame, ScreenShareCodec};

use crate::features::camera::{CameraCodec, EncodedCameraFrame};

use super::video_fragments::{self, OutboundVideoFrame};

#[path = "realtime_decode.rs"]
mod realtime_decode;

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

/// Входящий ретранслированный VP9 кадр видео.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InboundVideoFrame {
    /// Идентификатор целевой комнаты.
    pub(crate) room_id: String,
    /// Идентификатор аутентифицированного отправителя.
    pub(crate) sender_user_id: String,
    /// Локальная для отправителя последовательность пакетов.
    pub(crate) sequence: u64,
    /// Временная метка захвата или кодирования в микросекундах.
    pub(crate) timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub(crate) duration_us: u32,
    /// Сырые байты закодированного VP9 кадра.
    pub(crate) bytes: Vec<u8>,
    /// Может ли этот кадр открыть поток декодера.
    pub(crate) key_frame: bool,
}

/// Входящее событие остановки видеопотока участника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InboundVideoStreamEnded {
    /// Идентификатор сервера.
    pub(crate) server_id: String,
    /// Идентификатор целевой комнаты.
    pub(crate) room_id: String,
    /// Идентификатор пользователя, остановившего видеопоток.
    pub(crate) sender_user_id: String,
    /// Остановленный источник видео.
    pub(crate) source: VoiceVideoStreamSource,
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

/// Joins one direct-message voice call.
pub(crate) async fn join_direct_message_room(
    realtime: &RealtimeHandle,
    conversation_id: String,
) -> Result<VoiceRoomSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::JoinDirectMessageVoiceRoom),
            JoinDirectMessageVoiceRoom { conversation_id },
        )
        .await
}

/// Leaves one direct-message voice call.
pub(crate) async fn leave_direct_message_room(
    realtime: &RealtimeHandle,
    conversation_id: String,
) -> Result<VoiceRoomSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::LeaveDirectMessageVoiceRoom),
            LeaveDirectMessageVoiceRoom { conversation_id },
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
        .request_one_shot(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::ListServerVoiceRooms),
            ListServerVoiceRooms { server_id },
        )
        .await
}

/// Loads active direct-message voice call participant snapshots for the current user.
pub(crate) async fn list_direct_message_voice_rooms(
    realtime: &RealtimeHandle,
) -> Result<DirectMessageVoiceRoomsSnapshot, RealtimeError> {
    realtime
        .request_one_shot(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::ListDirectMessageVoiceRooms),
            ListDirectMessageVoiceRooms,
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
            let Some(snapshot) = realtime_decode::participants_changed(envelope) else {
                continue;
            };
            if sender.unbounded_send(snapshot).is_err() {
                break;
            }
        }
    });

    receiver
}

/// Подписывает текущую вкладку на события остановки видеопотоков участников.
pub(crate) fn subscribe_video_stream_ended(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<InboundVideoStreamEnded> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(event) = realtime_decode::video_stream_ended(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
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
            let Some(frame) = realtime_decode::voice_frame(&bytes) else {
                continue;
            };
            if sender.unbounded_send(frame).is_err() {
                break;
            }
        }
    });

    receiver
}

/// Подписывает текущую вкладку на входящие ретранслированные кадры демонстрации экрана.
pub(crate) fn subscribe_screen_frames(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<InboundVideoFrame> {
    let datagrams = realtime.subscribe_datagrams();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut datagrams = datagrams;
        let mut reassembler = video_fragments::VideoFrameReassembler::default();
        while let Some(bytes) = datagrams.next().await {
            let Some(datagram) = realtime_decode::screen_datagram(&bytes) else {
                continue;
            };
            let frame = if video_fragments::is_fragmented(&datagram) {
                reassembler
                    .push(datagram)
                    .map(realtime_decode::video_frame_from_datagram)
            } else {
                Some(realtime_decode::video_frame_from_datagram(datagram))
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

/// Подписывает текущую вкладку на входящие ретранслированные кадры камеры.
pub(crate) fn subscribe_camera_frames(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<InboundVideoFrame> {
    let datagrams = realtime.subscribe_datagrams();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut datagrams = datagrams;
        let mut reassembler = video_fragments::VideoFrameReassembler::default();
        while let Some(bytes) = datagrams.next().await {
            let Some(datagram) = realtime_decode::camera_datagram(&bytes) else {
                continue;
            };
            let frame = if video_fragments::is_fragmented(&datagram) {
                reassembler
                    .push(datagram)
                    .map(realtime_decode::video_frame_from_datagram)
            } else {
                Some(realtime_decode::video_frame_from_datagram(datagram))
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

/// Отправляет один закодированный кадр демонстрации экрана в активную голосовую комнату.
pub(crate) async fn send_screen_frame(
    realtime: &RealtimeHandle,
    _server_id: &str,
    room_id: &str,
    frame: EncodedScreenShareFrame,
) -> Result<(), RealtimeError> {
    let codec = match frame.codec {
        ScreenShareCodec::Vp9 => MediaCodec::Vp9,
    };
    let frame = OutboundVideoFrame {
        sequence: frame.sequence,
        timestamp_us: frame.timestamp_us,
        duration_us: frame.duration_us,
        key_frame: frame.key_frame,
        bytes: frame.bytes,
    };
    send_video_frame(
        realtime,
        room_id,
        MediaDatagramKind::ScreenFrame,
        codec,
        frame,
        "screen",
    )
    .await
}

/// Отправляет один закодированный кадр камеры в активную голосовую комнату.
pub(crate) async fn send_camera_frame(
    realtime: &RealtimeHandle,
    _server_id: &str,
    room_id: &str,
    frame: EncodedCameraFrame,
) -> Result<(), RealtimeError> {
    let codec = match frame.codec {
        CameraCodec::Vp9 => MediaCodec::Vp9,
    };
    let frame = OutboundVideoFrame {
        sequence: frame.sequence,
        timestamp_us: frame.timestamp_us,
        duration_us: frame.duration_us,
        key_frame: frame.key_frame,
        bytes: frame.bytes,
    };
    send_video_frame(
        realtime,
        room_id,
        MediaDatagramKind::CameraFrame,
        codec,
        frame,
        "camera",
    )
    .await
}

/// Сообщает о локальной остановке демонстрации экрана.
pub(crate) async fn send_screen_stream_stopped(
    realtime: &RealtimeHandle,
    server_id: &str,
    room_id: &str,
) -> Result<(), RealtimeError> {
    send_video_stream_stopped(
        realtime,
        server_id,
        room_id,
        VoiceVideoStreamSource::ScreenShare,
    )
    .await
}

/// Сообщает о локальной остановке камеры.
pub(crate) async fn send_camera_stream_stopped(
    realtime: &RealtimeHandle,
    server_id: &str,
    room_id: &str,
) -> Result<(), RealtimeError> {
    send_video_stream_stopped(realtime, server_id, room_id, VoiceVideoStreamSource::Camera).await
}

async fn send_video_frame(
    realtime: &RealtimeHandle,
    room_id: &str,
    kind: MediaDatagramKind,
    codec: MediaCodec,
    frame: OutboundVideoFrame,
    label: &str,
) -> Result<(), RealtimeError> {
    let room_id =
        Uuid::parse_str(room_id).map_err(|_| RealtimeError::new("Voice room id is invalid."))?;
    for datagram in video_fragments::video_frame_datagrams(room_id, kind, codec, frame)? {
        let bytes = datagram.encode().map_err(|error| {
            RealtimeError::new(format!("Failed to encode {label} frame: {error}"))
        })?;
        realtime.send_unreliable_bytes(Bytes::from(bytes)).await?;
    }

    Ok(())
}

async fn send_video_stream_stopped(
    realtime: &RealtimeHandle,
    server_id: &str,
    room_id: &str,
    source: VoiceVideoStreamSource,
) -> Result<(), RealtimeError> {
    realtime
        .send_reliable(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::StopVideoStream),
            StopVoiceVideoStream {
                server_id: server_id.to_owned(),
                room_id: room_id.to_owned(),
                source,
            },
        )
        .await
}
