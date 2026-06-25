//! Цикл получения ненадежных датаграмм WebTransport.

use std::time::Duration;

use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramKind};
use tokio::time::Instant;
use tracing::{debug, warn};
use uuid::Uuid;
use web_transport::Session;

use crate::features::voice_chat;
use crate::state::AppState;

const SLOW_MEDIA_DISPATCH_WARN_AFTER: Duration = Duration::from_millis(40);
const SLOW_MEDIA_WARNING_INTERVAL: Duration = Duration::from_secs(5);

/// Запускает читатель датаграмм для аутентифицированной сессии.
pub(crate) fn spawn_reader(state: AppState, session_id: Uuid, user_id: Uuid, session: Session) {
    tokio::spawn(async move {
        let mut last_slow_dispatch_warning_at = None;
        loop {
            let bytes = match session.recv_datagram().await {
                Ok(bytes) => bytes,
                Err(error) => {
                    debug!(
                        %session_id,
                        %user_id,
                        %error,
                        "WebTransport datagram reader closed"
                    );
                    break;
                }
            };

            match MediaDatagram::decode(&bytes) {
                Ok(datagram) => {
                    dispatch_with_warnings(
                        &state,
                        session_id,
                        user_id,
                        datagram,
                        &mut last_slow_dispatch_warning_at,
                    )
                    .await;
                }
                Err(error) => {
                    debug!(
                        %session_id,
                        %user_id,
                        %error,
                        bytes = bytes.len(),
                        "dropping invalid media datagram"
                    );
                }
            }
        }
    });
}

/// Обрабатывает датаграмму и предупреждает о задержках в горячем media path.
pub(crate) async fn dispatch_with_warnings(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
    last_slow_dispatch_warning_at: &mut Option<Instant>,
) {
    let kind = datagram.kind;
    let codec = datagram.codec;
    let room_id = datagram.room_id;
    let sequence = datagram.sequence;
    let timestamp_us = datagram.timestamp_us;
    let duration_us = datagram.duration_us;
    let payload_bytes = datagram.payload.len();
    let started_at = Instant::now();

    dispatch(state, session_id, user_id, datagram).await;

    let elapsed = started_at.elapsed();
    if elapsed >= SLOW_MEDIA_DISPATCH_WARN_AFTER
        && should_emit_slow_media_warning(last_slow_dispatch_warning_at, started_at)
    {
        // TODO: отправлять сообщение в телеграм(после появления пушей - администратору)
        warn!(
            %session_id,
            %user_id,
            %room_id,
            kind = ?kind,
            codec = ?codec,
            sequence,
            timestamp_us,
            duration_us,
            payload_bytes,
            elapsed_ms = elapsed.as_millis(),
            "slow realtime media datagram dispatch"
        );
    }
}

async fn dispatch(state: &AppState, session_id: Uuid, user_id: Uuid, datagram: MediaDatagram) {
    match datagram.kind {
        MediaDatagramKind::VoiceFrame if datagram.codec == MediaCodec::Opus => {
            voice_chat::media::handle_voice_frame(state, session_id, user_id, datagram).await;
        }
        MediaDatagramKind::ScreenFrame if datagram.codec == MediaCodec::Vp9 => {
            voice_chat::media::handle_screen_frame(state, session_id, user_id, datagram).await;
        }
        MediaDatagramKind::CameraFrame if datagram.codec == MediaCodec::Vp9 => {
            voice_chat::media::handle_camera_frame(state, session_id, user_id, datagram).await;
        }
        _ => {
            debug!(
                %session_id,
                %user_id,
                kind = ?datagram.kind,
                codec = ?datagram.codec,
                "dropping media datagram with unsupported kind/codec combination"
            );
        }
    }
}

fn should_emit_slow_media_warning(last_warning_at: &mut Option<Instant>, now: Instant) -> bool {
    if last_warning_at.is_some_and(|last_warning_at| {
        now.duration_since(last_warning_at) < SLOW_MEDIA_WARNING_INTERVAL
    }) {
        return false;
    }

    *last_warning_at = Some(now);
    true
}
