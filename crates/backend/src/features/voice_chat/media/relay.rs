//! Разбор и ретрансляция сырых WebTransport-медиадатаграмм.

use bytes::{Bytes, BytesMut};
use cheenhub_contracts::media::{
    MediaAudioSource, MediaCodec, MediaDatagram, MediaDatagramError, MediaDatagramKind,
};
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

/// Проверяет видеодорожку и подставляет ID аутентифицированной сессии для legacy-клиента.
/// У новых клиентов некорректный track ID запрещён; подмена не должна обходить FPS policy.
pub(super) fn authorized_video_header(
    mut header: MediaDatagramHeader,
    presence_session_id: Uuid,
    voice_protocol_version: u8,
) -> Option<MediaDatagramHeader> {
    if header.track_id != presence_session_id {
        if voice_protocol_version >= 5 {
            return None;
        }
        header.0.track_id = presence_session_id;
    }
    Some(header)
}

/// Проверяет принадлежность микрофона v5 аутентифицированной голосовой сессии.
/// Старые клиенты сохраняют собственные UUID захватов, системный звук независим.
pub(super) fn authorized_audio_track(
    audio_source: Option<MediaAudioSource>,
    track_id: Uuid,
    presence_session_id: Uuid,
    voice_protocol_version: u8,
) -> bool {
    if voice_protocol_version < 5 {
        return true;
    }
    match audio_source {
        Some(MediaAudioSource::Microphone) => track_id == presence_session_id,
        Some(MediaAudioSource::System) => true,
        None => false,
    }
}

/// Привязывает legacy-микрофон к аутентифицированной голосовой сессии.
///
/// Для v1 заполняет отсутствующие метаданные mono-микрофона; v2 сохраняет
/// число каналов. Системный звук имеет независимую дорожку.
pub(super) fn normalize_legacy_microphone_datagram(
    datagram: &mut MediaDatagram,
    presence_session_id: Uuid,
    voice_protocol_version: u8,
) -> bool {
    if voice_protocol_version >= 5
        || datagram.kind != MediaDatagramKind::VoiceFrame
        || datagram.codec != MediaCodec::Opus
        || datagram.audio_source == Some(MediaAudioSource::System)
    {
        return false;
    }
    let changed = datagram.track_id != presence_session_id
        || datagram.audio_source != Some(MediaAudioSource::Microphone);
    if changed {
        if datagram.audio_source.is_none() {
            datagram.audio_channels = 1;
        }
        datagram.track_id = presence_session_id;
        datagram.audio_source = Some(MediaAudioSource::Microphone);
    }
    changed
}

/// Готовит пакет для fanout, сохраняя zero-copy для подтверждённых дорожек.
///
/// У legacy-видео и микрофона переписывает UUID дорожки и при необходимости
/// конвертирует v1 в v2; во всех других случаях меняет только sender ID.
pub(super) fn prepare_relay_bytes(
    header: MediaDatagramHeader,
    bytes: Bytes,
    authenticated_user_id: Uuid,
    presence_session_id: Uuid,
    voice_protocol_version: u8,
) -> Result<Bytes, MediaDatagramError> {
    // v3 — формат серверной ретрансляции. Клиент не может назначить себе сессию.
    if header.voice_session_id.is_some() {
        return Err(MediaDatagramError::InvalidTrack);
    }
    let legacy_video = voice_protocol_version < 5
        && matches!(
            header.kind,
            MediaDatagramKind::ScreenFrame | MediaDatagramKind::CameraFrame
        )
        && header.track_id != presence_session_id;
    let legacy_microphone = voice_protocol_version < 5
        && header.kind == MediaDatagramKind::VoiceFrame
        && header.codec == MediaCodec::Opus
        && header.audio_source != Some(MediaAudioSource::System)
        && (header.track_id != presence_session_id
            || header.audio_source != Some(MediaAudioSource::Microphone));
    if !legacy_video && !legacy_microphone {
        return header.into_relay_bytes(bytes, authenticated_user_id);
    }

    let mut datagram = MediaDatagram::decode(&bytes)?;
    if legacy_video {
        datagram.track_id = presence_session_id;
    } else {
        normalize_legacy_microphone_datagram(
            &mut datagram,
            presence_session_id,
            voice_protocol_version,
        );
    }
    datagram.sender_user_id = authenticated_user_id;
    datagram.encode().map(Bytes::from)
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

    let Some(route) = active_media_route_for_transport(
        state,
        &header.room_id,
        &session_id,
        &user_id,
        allow_microphone_uplink,
        header.audio_source,
    )
    .await
    else {
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
    let legacy_recipients = route.legacy_recipients;
    let modern_recipients = route.modern_recipients;
    if media_kind == "voice"
        && !authorized_audio_track(
            header.audio_source,
            header.track_id,
            presence.session_id,
            presence.voice_protocol_version,
        )
    {
        debug!(%session_id, %user_id, track_id = %header.track_id,
            expected_session_id = %presence.session_id,
            "dropping voice datagram with mismatched authenticated session");
        return;
    }

    if let Some(allowed_video_presets) = allowed_video_presets {
        let Some(authorized_header) =
            authorized_video_header(header, presence.session_id, presence.voice_protocol_version)
        else {
            debug!(
                %session_id,
                %user_id,
                room_id = %header.room_id,
                track_id = %header.track_id,
                expected_track_id = %presence.session_id,
                media_kind,
                "dropping video datagram with track ID not matching authenticated session"
            );
            return;
        };
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
                .inspect_video_payload(
                    session_id,
                    &authorized_header,
                    payload,
                    allowed_video_presets,
                )
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

    let prepared = prepare_relay_bytes(
        header,
        bytes,
        user_id,
        presence.session_id,
        presence.voice_protocol_version,
    );
    let bytes = match prepared {
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
    super::fanout::fanout_relay_bytes(
        state,
        &legacy_recipients,
        &modern_recipients,
        presence.session_id,
        bytes,
    )
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

/// Определяет голосовую сессию только по аутентифицированному транспорту.
///
/// Отдельному microphone worker разрешается использовать лишь его серверную
/// привязку к основной голосовой сессии; для видео и системного аудио это запрещено.
pub(super) async fn active_media_route_for_transport(
    state: &AppState,
    room_id: &Uuid,
    transport_session_id: &Uuid,
    user_id: &Uuid,
    allow_microphone_uplink: bool,
    audio_source: Option<MediaAudioSource>,
) -> Option<MediaRouteSnapshot> {
    if let Some(route) =
        active_media_route_for_session(state, room_id, transport_session_id, user_id)
    {
        return Some(route);
    }
    if !allow_microphone_uplink || audio_source != Some(MediaAudioSource::Microphone) {
        return None;
    }
    let presence_session_id = state
        .voice_presence_store
        .microphone_uplink_presence_session(transport_session_id, user_id, room_id)
        .await?;
    active_media_route_for_session(state, room_id, &presence_session_id, user_id)
}

/// Ищет media route с presence и recipients из одной версии room snapshot.
///
/// Сначала проверяется серверная комната, затем комната личного звонка с тем
/// же идентификатором. `None` означает, что пользователь не состоит ни в одной
/// из поддерживаемых целей.
pub(super) fn active_media_route_for_session(
    state: &AppState,
    room_id: &Uuid,
    sender_session_id: &Uuid,
    user_id: &Uuid,
) -> Option<MediaRouteSnapshot> {
    state
        .voice_presence_store
        .media_route_for_session(
            VoicePresenceTargetKind::Server,
            room_id,
            sender_session_id,
            user_id,
        )
        .or_else(|| {
            state.voice_presence_store.media_route_for_session(
                VoicePresenceTargetKind::DirectMessage,
                room_id,
                sender_session_id,
                user_id,
            )
        })
}
