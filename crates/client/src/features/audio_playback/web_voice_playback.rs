//! Очистка browser-воспроизведения голосовых потоков.

use super::AudioPlaybackHandle;

impl AudioPlaybackHandle {
    /// Останавливает голосовое воспроизведение, не прерывая системные notification-звуки.
    pub(crate) fn stop_voice_playback(&self) {
        let sender_ids = {
            let inner = self.inner.borrow();
            let mut sender_ids = inner.senders.keys().cloned().collect::<Vec<_>>();
            for sender_id in inner.scheduled_sources.keys() {
                if !sender_ids.iter().any(|known_id| known_id == sender_id) {
                    sender_ids.push(sender_id.clone());
                }
            }
            for sender_id in inner.jitter_buffers.keys() {
                if !sender_ids.iter().any(|known_id| known_id == sender_id) {
                    sender_ids.push(sender_id.clone());
                }
            }
            sender_ids
        };
        for sender_id in sender_ids {
            self.stop_sender(&sender_id);
        }
    }
}
