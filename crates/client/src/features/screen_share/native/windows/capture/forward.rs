//! Передача готовых пакетов capture worker в runtime демонстрации.

use super::WorkerEvent;
use crate::features::screen_share::{EncodedScreenShareFrame, ScreenShareCodec};
use crate::features::video_encoding::EncodedVideoFrame;
use futures_channel::mpsc;

/// Передаёт пакеты по порядку и сообщает, доступен ли принимающий runtime.
pub(super) fn forward_packets(
    packets: Vec<EncodedVideoFrame>,
    sender: &mpsc::UnboundedSender<WorkerEvent>,
) -> bool {
    for packet in packets {
        if sender
            .unbounded_send(WorkerEvent::Frame(EncodedScreenShareFrame {
                sequence: packet.sequence,
                timestamp_us: packet.timestamp_us,
                duration_us: packet.duration_us,
                codec: ScreenShareCodec::Vp9,
                key_frame: packet.key_frame,
                width: packet.width,
                height: packet.height,
                bytes: packet.bytes,
            }))
            .is_err()
        {
            return false;
        }
    }
    true
}
