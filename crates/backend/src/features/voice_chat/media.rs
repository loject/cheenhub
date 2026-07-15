//! Обработка медиадатаграмм голосового чата.

use bytes::Bytes;
use cheenhub_contracts::media::MediaDatagram;
use cheenhub_contracts::video_presets::{
    BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS, VideoPresetId,
};
use tracing::{debug, warn};
use uuid::Uuid;

use super::infrastructure::VoicePresenceTargetKind;
use super::media_policy::{VideoAdmission, VideoDropReason};
use crate::state::AppState;

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
    let is_bound_microphone_uplink = allow_microphone_uplink
        && state
            .voice_presence_store
            .microphone_uplink_is_bound(
                &session_id,
                &user_id,
                &datagram.room_id,
                &presence.session_id,
            )
            .await;
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
            .inspect_video_datagram(session_id, user_id, &datagram, allowed_video_presets)
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
