//! Очистка browser-воспроизведения голосовых потоков.

use super::AudioPlaybackHandle;
use super::browser_helpers::{js_error_message, stop_audio_source};
use crate::features::audio_playback::tracks;
use dioxus::prelude::{debug, warn};

impl AudioPlaybackHandle {
    /// Останавливает дорожку системного звука вместе с демонстрацией экрана.
    pub(crate) fn stop_system_audio(&self, sender_user_id: &str, track_id: uuid::Uuid) {
        let key = self.inner.borrow_mut().tracks.stop_source(
            sender_user_id,
            cheenhub_contracts::media::MediaAudioSource::System,
            track_id,
        );
        if let Some(key) = key {
            self.stop_playback_key(&key);
            debug!(%sender_user_id, "stopped screen sharing audio playback");
        }
    }
    /// Останавливает все дорожки участника либо одну внутреннюю очередь.
    pub(crate) fn stop_sender(&self, sender_user_id: &str) {
        let keys = self
            .inner
            .borrow()
            .jitter_buffers
            .keys()
            .filter(|key| {
                key.as_str() == sender_user_id || tracks::playback_owner(key) == sender_user_id
            })
            .cloned()
            .collect::<Vec<_>>();
        self.inner.borrow_mut().tracks.remove_sender(sender_user_id);
        for key in keys {
            self.stop_playback_key(&key);
        }
    }

    /// Освобождает декодер и очереди одной дорожки, сохраняя историю смены захвата.
    pub(super) fn stop_playback_key(&self, sender_user_id: &str) {
        let (sender, sources) = {
            let mut inner = self.inner.borrow_mut();
            let sender = inner.senders.remove(sender_user_id);
            if let Some(sender) = &sender {
                sender.pause.cancel();
            }
            inner.jitter_buffers.remove(sender_user_id);
            inner.jitter_warning_at_ms.remove(sender_user_id);
            inner.decoder_queue_warning_at_ms.remove(sender_user_id);
            inner.playback_schedule_warning_at.remove(sender_user_id);
            inner.diagnostics.remove_sender(sender_user_id);
            let sources = inner
                .scheduled_sources
                .remove(sender_user_id)
                .unwrap_or_default();
            inner.scheduled_until.remove(sender_user_id);
            (sender, sources)
        };

        for scheduled in sources {
            if let Err(error) = stop_audio_source(&scheduled.source) {
                warn!(
                    error = %js_error_message(error),
                    %sender_user_id,
                    "failed to stop scheduled audio source"
                );
            }
        }

        if let Some(sender) = sender
            && let Err(error) = sender.decoder.close()
        {
            warn!(
                error = %js_error_message(error),
                %sender_user_id,
                "failed to close audio decoder"
            );
        }
    }
    /// Останавливает голосовое воспроизведение, не прерывая системные notification-звуки.
    pub(crate) fn stop_voice_playback(&self) {
        self.inner.borrow_mut().tracks.clear();
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
