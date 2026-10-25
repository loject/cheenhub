//! Доставка видеокадров и ограниченный стартовый буфер до подключения плитки.

use dioxus::prelude::{debug, warn};
use futures_channel::mpsc;

use super::ParticipantVideoFrame;

const MAX_PENDING_FRAMES: usize = 128;
const MAX_PENDING_BYTES: usize = 8 * 1024 * 1024;

/// Сохраняет декодируемую последовательность до подписки и передаёт кадры активным плиткам.
#[derive(Default)]
pub(crate) struct ParticipantVideoDelivery {
    subscribers: Vec<mpsc::UnboundedSender<ParticipantVideoFrame>>,
    pending: Vec<ParticipantVideoFrame>,
    pending_bytes: usize,
}

#[cfg(test)]
mod tests;

impl ParticipantVideoDelivery {
    /// Передаёт кадр подписчикам либо буферизует его, начиная с ключевого кадра.
    pub(super) fn publish(&mut self, frame: ParticipantVideoFrame) {
        self.subscribers
            .retain(|subscriber| subscriber.unbounded_send(frame.clone()).is_ok());
        if !self.subscribers.is_empty() {
            return;
        }
        if frame.key_frame {
            self.pending.clear();
            self.pending_bytes = 0;
            debug!(user_id = %frame.sender_user_id, sequence = frame.sequence,
                "buffering video key frame until participant tile subscribes");
        } else if self.pending.is_empty() {
            return;
        }
        if self.pending.len() >= MAX_PENDING_FRAMES
            || frame.bytes.len() > MAX_PENDING_BYTES.saturating_sub(self.pending_bytes)
        {
            self.pending.clear();
            self.pending_bytes = 0;
            warn!(user_id = %frame.sender_user_id, sequence = frame.sequence,
                "participant video startup buffer exceeded limit; waiting for key frame");
            return;
        }
        self.pending_bytes += frame.bytes.len();
        self.pending.push(frame);
    }

    /// Подключает плитку, передавая ей сохранённый ключевой кадр и все последующие кадры.
    pub(super) fn subscribe(&mut self) -> mpsc::UnboundedReceiver<ParticipantVideoFrame> {
        let (sender, receiver) = mpsc::unbounded();
        for frame in self.pending.drain(..) {
            let _ = sender.unbounded_send(frame);
        }
        self.pending_bytes = 0;
        self.subscribers.push(sender);
        receiver
    }
}
