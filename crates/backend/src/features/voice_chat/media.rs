//! Обработка медиадатаграмм голосового чата.

use bytes::Bytes;
use cheenhub_contracts::media::MediaDatagram;
use tracing::{debug, warn};
use uuid::Uuid;

use super::infrastructure::VoicePresenceTargetKind;
use crate::state::AppState;

/// Обрабатывает одну декодированную медиадатаграмму голоса.
pub(crate) async fn handle_voice_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
) {
    handle_room_media_frame(state, session_id, user_id, datagram, "voice", true).await;
}

/// Обрабатывает одну декодированную медиадатаграмму демонстрации экрана.
pub(crate) async fn handle_screen_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
) {
    handle_room_media_frame(state, session_id, user_id, datagram, "screen", false).await;
}

/// Обрабатывает одну декодированную медиадатаграмму камеры.
pub(crate) async fn handle_camera_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
) {
    handle_room_media_frame(state, session_id, user_id, datagram, "camera", false).await;
}

async fn handle_room_media_frame(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    mut datagram: MediaDatagram,
    media_kind: &'static str,
    allow_microphone_uplink: bool,
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
