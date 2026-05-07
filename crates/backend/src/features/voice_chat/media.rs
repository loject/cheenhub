//! Voice chat media datagram handling.

use cheenhub_contracts::media::MediaDatagram;
use tracing::debug;
use uuid::Uuid;

/// Handles one decoded voice media datagram.
pub(crate) fn handle_voice_frame(session_id: Uuid, user_id: Uuid, datagram: MediaDatagram) {
    debug!(
        %session_id,
        %user_id,
        room_id = %datagram.room_id,
        sequence = datagram.sequence,
        timestamp_us = datagram.timestamp_us,
        duration_us = datagram.duration_us,
        payload_bytes = datagram.payload.len(),
        codec = ?datagram.codec,
        "received voice media datagram"
    );
}
