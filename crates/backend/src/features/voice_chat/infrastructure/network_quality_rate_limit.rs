//! Ограничение частоты публикации сетевых метрик голосового общения.

use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use uuid::Uuid;

use super::{InMemoryVoicePresenceStore, VoicePresence};

const MIN_NETWORK_QUALITY_PUBLISH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Default)]
pub(super) struct NetworkQualityRateLimiter {
    last_accepted_at: HashMap<Uuid, Instant>,
}

impl NetworkQualityRateLimiter {
    fn allow(&mut self, realtime_stream_id: Uuid, now: Instant) -> bool {
        if self
            .last_accepted_at
            .get(&realtime_stream_id)
            .is_some_and(|last_accepted_at| {
                now.saturating_duration_since(*last_accepted_at)
                    < MIN_NETWORK_QUALITY_PUBLISH_INTERVAL
            })
        {
            return false;
        }
        self.last_accepted_at.insert(realtime_stream_id, now);
        true
    }

    fn remove_presences(&mut self, removed: &[VoicePresence]) {
        for presence in removed {
            self.last_accepted_at.remove(&presence.realtime_stream_id);
        }
    }
}

impl InMemoryVoicePresenceStore {
    /// Проверяет частоту публикации RTT для одного аутентифицированного realtime-потока.
    pub(crate) async fn allow_network_quality_publish_at(
        &self,
        realtime_stream_id: Uuid,
        now: Instant,
    ) -> bool {
        self.network_quality_rate_limiter
            .lock()
            .await
            .allow(realtime_stream_id, now)
    }

    pub(super) async fn clear_network_quality_rate_limits_for(&self, removed: &[VoicePresence]) {
        self.network_quality_rate_limiter
            .lock()
            .await
            .remove_presences(removed);
    }

    #[cfg(test)]
    /// Возвращает число сборов участников для проверки порядка rate limit.
    pub(crate) fn room_participants_call_count(&self) -> usize {
        self.room_participants_calls.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use uuid::Uuid;

    use super::super::InMemoryVoicePresenceStore;

    #[tokio::test]
    async fn allows_normal_voice_cadence_and_rejects_faster_publications() {
        let store = InMemoryVoicePresenceStore::default();
        let stream_id = Uuid::new_v4();
        let started_at = Instant::now();

        assert!(
            store
                .allow_network_quality_publish_at(stream_id, started_at)
                .await
        );
        assert!(
            !store
                .allow_network_quality_publish_at(
                    stream_id,
                    started_at + Duration::from_millis(499),
                )
                .await
        );
        assert!(
            store
                .allow_network_quality_publish_at(
                    stream_id,
                    started_at + Duration::from_millis(750),
                )
                .await
        );
    }
}
