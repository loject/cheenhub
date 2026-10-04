//! Обработка медиадатаграмм голосового чата.

use bytes::{Bytes, BytesMut};
use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramError, MediaDatagramKind};
use cheenhub_contracts::video_presets::{
    BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS, VideoPresetId,
};
use tracing::{debug, warn};
use uuid::Uuid;

use super::infrastructure::VoicePresenceTargetKind;
use super::media_policy::{VideoAdmission, VideoDropReason};
use crate::state::AppState;

#[cfg(test)]
mod tests;

const MEDIA_DATAGRAM_HEADER_LEN: usize = 64;
const MEDIA_DATAGRAM_SENDER_USER_ID_START: usize = 44;
const MEDIA_DATAGRAM_SENDER_USER_ID_END: usize = 60;

/// Заголовок wire-медиадатаграммы, разобранный без копирования payload.
///
/// Парсер проверяет сигнатуру, версию, значения kind/codec и объявленную длину.
/// Дополнительные trailing bytes принимаются, как и в текущем owned decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MediaDatagramHeader {
    /// Вид медиадатаграммы.
    pub(crate) kind: MediaDatagramKind,
    /// Кодек закодированной полезной нагрузки.
    pub(crate) codec: MediaCodec,
    /// Флаги wire-заголовка.
    pub(crate) flags: u8,
    /// Локальная для отправителя последовательность пакетов.
    pub(crate) sequence: u64,
    /// Временная метка захвата или кодирования в микросекундах.
    pub(crate) timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub(crate) duration_us: u32,
    /// Идентификатор целевой комнаты.
    pub(crate) room_id: Uuid,
    /// Sender ID, полученный из недоверенного wire-заголовка.
    pub(crate) sender_user_id: Uuid,
    /// Объявленная заголовком длина payload.
    pub(crate) payload_len: usize,
    wire_len: usize,
}

impl MediaDatagramHeader {
    /// Разбирает один wire-заголовок без копирования payload.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку для неверной сигнатуры, неподдерживаемой версии, kind,
    /// codec или усечённой датаграммы.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, MediaDatagramError> {
        if bytes.len() < MEDIA_DATAGRAM_HEADER_LEN {
            return Err(MediaDatagramError::Truncated);
        }
        if &bytes[..4] != b"CHUB" {
            return Err(MediaDatagramError::BadMagic);
        }
        if bytes[4] != 1 {
            return Err(MediaDatagramError::UnknownVersion(bytes[4]));
        }
        let kind = match bytes[5] {
            1 => MediaDatagramKind::VoiceFrame,
            2 => MediaDatagramKind::ScreenFrame,
            3 => MediaDatagramKind::CameraFrame,
            value => return Err(MediaDatagramError::UnknownKind(value)),
        };
        let codec = match bytes[6] {
            1 => MediaCodec::Opus,
            2 => MediaCodec::Vp9,
            value => return Err(MediaDatagramError::UnknownCodec(value)),
        };

        let payload_len = u32::from_be_bytes(copy_array(&bytes[60..64])) as usize;
        let wire_len = MEDIA_DATAGRAM_HEADER_LEN
            .checked_add(payload_len)
            .ok_or(MediaDatagramError::PayloadTooLarge(payload_len))?;
        if bytes.len() < wire_len {
            return Err(MediaDatagramError::Truncated);
        }

        Ok(Self {
            kind,
            codec,
            flags: bytes[7],
            sequence: u64::from_be_bytes(copy_array(&bytes[8..16])),
            timestamp_us: u64::from_be_bytes(copy_array(&bytes[16..24])),
            duration_us: u32::from_be_bytes(copy_array(&bytes[24..28])),
            room_id: Uuid::from_bytes(copy_array(&bytes[28..44])),
            sender_user_id: Uuid::from_bytes(copy_array(&bytes[44..60])),
            payload_len,
            wire_len,
        })
    }

    /// Возвращает объявленный payload как slice исходного wire-буфера.
    ///
    /// # Errors
    ///
    /// Возвращает `Truncated`, если буфер короче объявленной датаграммы.
    pub(crate) fn payload(self, bytes: &[u8]) -> Result<&[u8], MediaDatagramError> {
        if bytes.len() < self.wire_len {
            return Err(MediaDatagramError::Truncated);
        }
        Ok(&bytes[MEDIA_DATAGRAM_HEADER_LEN..self.wire_len])
    }

    /// Возвращает длину заголовка вместе с объявленным payload.
    pub(crate) fn wire_len(self) -> usize {
        self.wire_len
    }

    fn into_relay_bytes(
        self,
        bytes: Bytes,
        sender_user_id: Uuid,
    ) -> Result<Bytes, MediaDatagramError> {
        if bytes.len() < self.wire_len {
            return Err(MediaDatagramError::Truncated);
        }

        let mut bytes = match bytes.try_into_mut() {
            Ok(bytes) => bytes,
            Err(bytes) => BytesMut::from(bytes.as_ref()),
        };
        bytes.truncate(self.wire_len());
        bytes[MEDIA_DATAGRAM_SENDER_USER_ID_START..MEDIA_DATAGRAM_SENDER_USER_ID_END]
            .copy_from_slice(sender_user_id.as_bytes());
        Ok(bytes.freeze())
    }
}

fn copy_array<const N: usize>(slice: &[u8]) -> [u8; N] {
    let mut array = [0; N];
    array.copy_from_slice(slice);
    array
}

/// Ретранслирует voice/Opus datagram без декодирования и повторного кодирования payload.
pub(crate) async fn handle_voice_frame_bytes(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    bytes: Bytes,
    header: MediaDatagramHeader,
) {
    let payload_bytes = match header.payload(&bytes) {
        Ok(payload) => payload.len(),
        Err(error) => {
            debug!(
                %session_id,
                %user_id,
                room_id = %header.room_id,
                %error,
                "dropping invalid voice datagram"
            );
            return;
        }
    };
    debug!(
        %session_id,
        %user_id,
        room_id = %header.room_id,
        media_kind = "voice",
        sequence = header.sequence,
        timestamp_us = header.timestamp_us,
        duration_us = header.duration_us,
        payload_bytes,
        "received voice room media datagram"
    );

    let Some(presence) = active_presence_for_user(state, &header.room_id, &user_id).await else {
        debug!(
            %session_id,
            %user_id,
            room_id = %header.room_id,
            media_kind = "voice",
            "dropping media datagram from user outside target room"
        );
        return;
    };

    let is_presence_session = presence.session_id == session_id;
    let is_bound_microphone_uplink = if is_presence_session {
        false
    } else {
        state
            .voice_presence_store
            .microphone_uplink_is_bound(
                &session_id,
                &user_id,
                &header.room_id,
                &presence.session_id,
            )
            .await
    };
    if !is_presence_session && !is_bound_microphone_uplink {
        debug!(
            %session_id,
            expected_session_id = %presence.session_id,
            %user_id,
            room_id = %header.room_id,
            media_kind = "voice",
            "dropping media datagram from unauthorized session"
        );
        return;
    }

    let recipients = state
        .voice_presence_store
        .media_recipient_sessions(presence.target_kind, &header.room_id, &presence.session_id)
        .await;
    if recipients.is_empty() {
        return;
    }

    let bytes = match header.into_relay_bytes(bytes, user_id) {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!(
                %session_id,
                %user_id,
                room_id = %header.room_id,
                media_kind = "voice",
                %error,
                "failed to prepare zero-copy voice datagram relay"
            );
            return;
        }
    };
    state
        .realtime_hub
        .fanout_datagram_to_sessions(&recipients, bytes)
        .await;
}

/// Обрабатывает одну декодированную медиадатаграмму голоса.
pub(crate) async fn handle_voice_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
) {
    handle_room_media_frame(state, session_id, user_id, datagram, "voice", true, None).await;
}

/// Обрабатывает одну декодированную медиадатаграмму демонстрации экрана.
pub(crate) async fn handle_screen_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
) {
    handle_room_media_frame(
        state,
        session_id,
        user_id,
        datagram,
        "screen",
        false,
        Some(BASE_SCREEN_SHARE_VIDEO_PRESETS),
    )
    .await;
}

/// Обрабатывает одну декодированную медиадатаграмму камеры.
pub(crate) async fn handle_camera_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
) {
    handle_room_media_frame(
        state,
        session_id,
        user_id,
        datagram,
        "camera",
        false,
        Some(BASE_CAMERA_VIDEO_PRESETS),
    )
    .await;
}

async fn handle_room_media_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    mut datagram: MediaDatagram,
    media_kind: &'static str,
    allow_microphone_uplink: bool,
    allowed_video_presets: Option<&'static [VideoPresetId]>,
) {
    debug!(
        %session_id,
        %user_id,
        room_id = %datagram.room_id,
        media_kind,
        sequence = datagram.sequence,
        timestamp_us = datagram.timestamp_us,
        duration_us = datagram.duration_us,
        payload_bytes = datagram.payload.len(),
        codec = ?datagram.codec,
        "received voice room media datagram"
    );

    let Some(presence) = active_presence_for_user(state, &datagram.room_id, &user_id).await else {
        debug!(
            %session_id,
            %user_id,
            room_id = %datagram.room_id,
            media_kind,
            "dropping media datagram from user outside target room"
        );
        return;
    };
    let is_presence_session = presence.session_id == session_id;
    let is_bound_microphone_uplink = if is_presence_session || !allow_microphone_uplink {
        false
    } else {
        state
            .voice_presence_store
            .microphone_uplink_is_bound(
                &session_id,
                &user_id,
                &datagram.room_id,
                &presence.session_id,
            )
            .await
    };
    if !is_presence_session && !is_bound_microphone_uplink {
        debug!(
            %session_id,
            expected_session_id = %presence.session_id,
            %user_id,
            room_id = %datagram.room_id,
            media_kind,
            "dropping media datagram from unauthorized session"
        );
        return;
    }

    if let Some(allowed_video_presets) = allowed_video_presets {
        let admission = state
            .voice_presence_store
            .inspect_video_datagram(session_id, &datagram, allowed_video_presets)
            .await;
        if !video_admission_allows_fanout(
            admission,
            session_id,
            user_id,
            datagram.room_id,
            media_kind,
            datagram.sequence,
        ) {
            return;
        }
    }

    datagram.sender_user_id = user_id;
    let recipients = state
        .voice_presence_store
        .media_recipient_sessions(
            presence.target_kind,
            &datagram.room_id,
            &presence.session_id,
        )
        .await;
    if recipients.is_empty() {
        return;
    }

    let bytes = match datagram.encode() {
        Ok(bytes) => Bytes::from(bytes),
        Err(error) => {
            warn!(
                %session_id,
                %user_id,
                room_id = %datagram.room_id,
                media_kind,
                %error,
                "failed to encode relayed media datagram"
            );
            return;
        }
    };
    state
        .realtime_hub
        .fanout_datagram_to_sessions(&recipients, bytes)
        .await;
}

fn video_admission_allows_fanout(
    admission: VideoAdmission,
    session_id: Uuid,
    user_id: Uuid,
    room_id: Uuid,
    media_kind: &'static str,
    sequence: u64,
) -> bool {
    let VideoAdmission::Drop(reason) = admission else {
        return true;
    };
    match reason {
        VideoDropReason::UnsupportedResolution { width, height } => warn!(
            %session_id,
            %user_id,
            %room_id,
            media_kind,
            sequence,
            width,
            height,
            "blocked video publication with unsupported resolution"
        ),
        VideoDropReason::FpsLimitExceeded {
            max_fps,
            observed_frames,
        } => warn!(
            %session_id,
            %user_id,
            %room_id,
            media_kind,
            sequence,
            max_fps,
            observed_frames,
            "blocked video publication after sustained FPS limit violation"
        ),
        VideoDropReason::InvalidVp9KeyFrame | VideoDropReason::MalformedFragment => warn!(
            %session_id,
            %user_id,
            %room_id,
            media_kind,
            sequence,
            reason = ?reason,
            "blocked malformed video publication datagram"
        ),
        VideoDropReason::AwaitingFirstFragment
        | VideoDropReason::AwaitingKeyFrame
        | VideoDropReason::FpsBlockActive => debug!(
            %session_id,
            %user_id,
            %room_id,
            media_kind,
            sequence,
            reason = ?reason,
            "dropping video datagram while publication is blocked"
        ),
    }
    false
}

async fn active_presence_for_user(
    state: &AppState,
    room_id: &Uuid,
    user_id: &Uuid,
) -> Option<super::infrastructure::VoicePresence> {
    if let Some(presence) = state
        .voice_presence_store
        .room_presence_for_user(VoicePresenceTargetKind::Server, room_id, user_id)
        .await
    {
        return Some(presence);
    }
    state
        .voice_presence_store
        .room_presence_for_user(VoicePresenceTargetKind::DirectMessage, room_id, user_id)
        .await
}
