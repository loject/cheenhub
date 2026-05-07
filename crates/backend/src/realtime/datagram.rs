//! WebTransport unreliable datagram receive loop.

use cheenhub_contracts::media::{MediaDatagram, MediaDatagramKind};
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

async fn dispatch(state: &AppState, session_id: Uuid, user_id: Uuid, datagram: MediaDatagram) {
    match datagram.kind {
        MediaDatagramKind::VoiceFrame => {
            voice_chat::media::handle_voice_frame(state, session_id, user_id, datagram).await;
        }
    }
}
