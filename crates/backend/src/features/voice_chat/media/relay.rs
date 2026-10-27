//! Разбор и ретрансляция сырых WebTransport-медиадатаграмм.

use bytes::{Bytes, BytesMut};
use cheenhub_contracts::media::{MediaCodec, MediaDatagramError, MediaDatagramKind};
use cheenhub_contracts::video_presets::{
    BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS,
};
use tracing::{debug, warn};
use uuid::Uuid;

use super::super::infrastructure::{MediaRouteSnapshot, VoicePresenceTargetKind};
use super::super::media_policy::{VideoAdmission, VideoDropReason};
use crate::state::AppState;

const MEDIA_DATAGRAM_SENDER_USER_ID_START: usize = 44;
const MEDIA_DATAGRAM_SENDER_USER_ID_END: usize = 60;

/// Заголовок общего медиаконтракта с серверной операцией замены sender ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MediaDatagramHeader(cheenhub_contracts::media::MediaDatagramHeader);

impl std::ops::Deref for MediaDatagramHeader {
    type Target = cheenhub_contracts::media::MediaDatagramHeader;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MediaDatagramHeader {
    /// Проверяет v1/v2 общим parser без копирования payload.
    ///
    /// # Errors
    /// Возвращает ошибку wire-формата или метаданных дорожки.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, MediaDatagramError> {
        cheenhub_contracts::media::MediaDatagramHeader::decode(bytes).map(Self)
    }

    /// Подставляет аутентифицированного отправителя, сохраняя версию и дорожку.
    ///
    /// # Errors
    /// Возвращает `Truncated`, если исходный буфер короче объявленной длины.
    pub(super) fn into_relay_bytes(
        self,
        bytes: Bytes,
        authenticated_user_id: Uuid,
    ) -> Result<Bytes, MediaDatagramError> {
        if bytes.len() < self.wire_len() {
            return Err(MediaDatagramError::Truncated);
        }
        let mut bytes = match bytes.try_into_mut() {
            Ok(bytes) => bytes,
            Err(bytes) => BytesMut::from(bytes.as_ref()),
        };
        bytes.truncate(self.wire_len());
        bytes[MEDIA_DATAGRAM_SENDER_USER_ID_START..MEDIA_DATAGRAM_SENDER_USER_ID_END]
            .copy_from_slice(authenticated_user_id.as_bytes());
        Ok(bytes.freeze())
    }
}

/// Ретранслирует WebTransport voice и video datagrams исходным `Bytes`.
///
/// VP9 encoded bytes проходят существующую admission policy через borrowed
/// slice; Opus не декодируется. Перед fanout sender ID заменяется ID из auth.
pub(crate) async fn handle_webtransport_frame_bytes(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    bytes: Bytes,
    header: MediaDatagramHeader,
) {
    let (media_kind, allow_microphone_uplink, allowed_video_presets) =
        match (header.kind, header.codec) {
            (MediaDatagramKind::VoiceFrame, MediaCodec::Opus) => ("voice", true, None),
            (MediaDatagramKind::ScreenFrame, MediaCodec::Vp9) => {
                ("screen", false, Some(BASE_SCREEN_SHARE_VIDEO_PRESETS))
            }
            (MediaDatagramKind::CameraFrame, MediaCodec::Vp9) => {
                ("camera", false, Some(BASE_CAMERA_VIDEO_PRESETS))
            }
            _ => {
                debug!(
                    %session_id,
                    %user_id,
                    kind = ?header.kind,
                    codec = ?header.codec,
                    "dropping media datagram with unsupported kind/codec combination"
                );
                return;
            }
        };

    let payload_bytes = match header.payload(&bytes) {
        Ok(payload) => payload.len(),
        Err(error) => {
            debug!(%session_id, %user_id, %error, "dropping invalid media datagram");
            return;
        }
    };
    debug!(
        %session_id,
        %user_id,
        room_id = %header.room_id,
        media_kind,
        track_id = %header.track_id,
        audio_source = ?header.audio_source,
        sequence = header.sequence,
        timestamp_us = header.timestamp_us,
        duration_us = header.duration_us,
        payload_bytes,
        codec = ?header.codec,
        "received voice room media datagram"
    );

    let Some(route) = active_media_route_for_user(state, &header.room_id, &user_id) else {
        debug!(
            %session_id,
            %user_id,
            room_id = %header.room_id,
            media_kind,
            "dropping media datagram from user outside target room"
        );
        return;
    };
    let presence = route.presence;
    let recipients = route.recipients;

    let is_presence_session = presence.session_id == session_id;
    let is_bound_microphone_uplink = if is_presence_session
        || !allow_microphone_uplink
        || header.audio_source == Some(cheenhub_contracts::media::MediaAudioSource::System)
    {
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
            media_kind,
            "dropping media datagram from unauthorized session"
        );
        return;
    }

    if let Some(allowed_video_presets) = allowed_video_presets {
        let admission = {
            let payload = match header.payload(&bytes) {
                Ok(payload) => payload,
                Err(error) => {
                    debug!(%session_id, %user_id, %error, "dropping invalid video datagram");
                    return;
                }
            };
            state
                .voice_presence_store
                .inspect_video_payload(session_id, &header, payload, allowed_video_presets)
                .await
        };
        if !video_admission_allows_fanout(
            admission,
            session_id,
            user_id,
            header.room_id,
            media_kind,
            header.sequence,
        ) {
            return;
        }
    }

    if recipients.is_empty() || (recipients.len() == 1 && recipients[0] == presence.session_id) {
        return;
    }

    let bytes = match header.into_relay_bytes(bytes, user_id) {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!(
                %session_id,
                %user_id,
                room_id = %header.room_id,
                media_kind,
                %error,
                "failed to prepare zero-copy media datagram relay"
            );
            return;
        }
    };
    state
        .realtime_hub
        .fanout_datagram_to_sessions_except(&recipients, presence.session_id, bytes)
        .await;
}

/// Проверяет, должна ли существующая video policy передать кадр дальше.
pub(super) fn video_admission_allows_fanout(
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

/// Ищет media route с presence и recipients из одной версии room snapshot.
///
/// Сначала проверяется серверная комната, затем комната личного звонка с тем
/// же идентификатором. `None` означает, что пользователь не состоит ни в одной
/// из поддерживаемых целей.
pub(super) fn active_media_route_for_user(
    state: &AppState,
    room_id: &Uuid,
    user_id: &Uuid,
) -> Option<MediaRouteSnapshot> {
    state
        .voice_presence_store
        .media_route(VoicePresenceTargetKind::Server, room_id, user_id)
        .or_else(|| {
            state.voice_presence_store.media_route(
                VoicePresenceTargetKind::DirectMessage,
                room_id,
                user_id,
            )
        })
}
