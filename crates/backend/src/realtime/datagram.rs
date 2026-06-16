//! Цикл получения ненадежных датаграмм WebTransport.

use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramKind};
use tracing::debug;
use uuid::Uuid;
use web_transport::Session;

use crate::features::voice_chat;
use crate::state::AppState;

/// Spawns the authenticated session datagram reader.
pub(crate) fn spawn_reader(state: AppState, session_id: Uuid, user_id: Uuid, session: Session) {
    tokio::spawn(async move {
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
                Ok(datagram) => dispatch(&state, session_id, user_id, datagram).await,
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

pub(crate) async fn dispatch(
    state: &AppState,
    session_id: Uuid,
    user_id: Uuid,
    datagram: MediaDatagram,
) {
    match datagram.kind {
        MediaDatagramKind::VoiceFrame if datagram.codec == MediaCodec::Opus => {
            voice_chat::media::handle_voice_frame(state, session_id, user_id, datagram).await;
        }
        MediaDatagramKind::ScreenFrame if datagram.codec == MediaCodec::Vp9 => {
            voice_chat::media::handle_screen_frame(state, session_id, user_id, datagram).await;
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
