//! Обработка медиадатаграмм голосового чата.

use bytes::Bytes;
use cheenhub_contracts::media::MediaDatagram;
use cheenhub_contracts::video_presets::{
    BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS, VideoPresetId,
};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::state::AppState;

mod fanout;
mod relay;
#[cfg(test)]
mod tests;

pub(crate) use relay::{MediaDatagramHeader, handle_webtransport_frame_bytes};
use relay::{
    active_media_route_for_transport, authorized_audio_track, video_admission_allows_fanout,
};

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
        track_id = %datagram.track_id,
        audio_source = ?datagram.audio_source,
        sequence = datagram.sequence,
        timestamp_us = datagram.timestamp_us,
        duration_us = datagram.duration_us,
        payload_bytes = datagram.payload.len(),
        codec = ?datagram.codec,
        "received voice room media datagram"
    );

    let Some(route) = active_media_route_for_transport(
        state,
        &datagram.room_id,
        &session_id,
        &user_id,
        allow_microphone_uplink,
        datagram.audio_source,
    )
    .await
    else {
        debug!(
            %session_id,
            %user_id,
            room_id = %datagram.room_id,
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
            datagram.audio_source,
            datagram.track_id,
            presence.session_id,
            presence.voice_protocol_version,
        )
    {
        debug!(%session_id, %user_id, track_id = %datagram.track_id,
            expected_session_id = %presence.session_id,
            "dropping voice datagram with mismatched authenticated session");
        return;
    }
    if relay::normalize_legacy_microphone_datagram(
        &mut datagram,
        presence.session_id,
        presence.voice_protocol_version,
    ) {
        debug!(
            %session_id,
            %user_id,
            authenticated_voice_session_id = %presence.session_id,
            "дорожка legacy-микрофона привязана к голосовой сессии"
        );
    }
    if let Some(allowed_video_presets) = allowed_video_presets {
        if datagram.track_id != presence.session_id {
            if presence.voice_protocol_version >= 5 {
                debug!(
                    %session_id,
                    %user_id,
                    room_id = %datagram.room_id,
                    track_id = %datagram.track_id,
                    expected_track_id = %presence.session_id,
                    media_kind,
                    "dropping video datagram with track ID not matching authenticated session"
                );
                return;
            }
            // Старые клиенты публиковали захват вместо UUID сессии.
            // Нормализуем до admission policy, чтобы нельзя было обойти её сменой track ID.
            datagram.track_id = presence.session_id;
        }
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
    if recipients.is_empty() || (recipients.len() == 1 && recipients[0] == presence.session_id) {
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
    fanout::fanout_relay_bytes(
        state,
        &legacy_recipients,
        &modern_recipients,
        presence.session_id,
        bytes,
    )
    .await;
}
