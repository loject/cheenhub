//! Политика публикации RTT активного голосового соединения.

const SIGNIFICANT_RTT_CHANGE_MS: u32 = 25;
const RTT_HEARTBEAT_MS: u64 = 1_000;

/// Категория RTT, влияющая на визуальную оценку качества.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VoiceRttQuality {
    /// Задержка комфортна для разговора.
    Good,
    /// Задержка заметна, но разговор ещё возможен.
    Degraded,
    /// Задержка делает разговор практически непригодным.
    Poor,
}

/// Возвращает категорию качества для измеренного RTT.
pub(super) fn rtt_quality(rtt_ms: u32) -> VoiceRttQuality {
    match rtt_ms {
        0..=150 => VoiceRttQuality::Good,
        151..=500 => VoiceRttQuality::Degraded,
        _ => VoiceRttQuality::Poor,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct NetworkQualityPublicationState {
    last_rtt_ms: Option<u32>,
    last_published_at_ms: Option<u64>,
}

impl NetworkQualityPublicationState {
    pub(super) fn should_publish(self, rtt_ms: u32, now_ms: u64) -> bool {
        let Some(previous_rtt_ms) = self.last_rtt_ms else {
            return true;
        };
        let bucket_changed = rtt_quality(previous_rtt_ms) != rtt_quality(rtt_ms);
        let changed_significantly = previous_rtt_ms.abs_diff(rtt_ms) >= SIGNIFICANT_RTT_CHANGE_MS;
        let heartbeat_due = self.last_published_at_ms.is_none_or(|published_at_ms| {
            now_ms.saturating_sub(published_at_ms) >= RTT_HEARTBEAT_MS
        });
        bucket_changed || changed_significantly || heartbeat_due
    }

    pub(super) fn mark_published(&mut self, rtt_ms: u32, now_ms: u64) {
        self.last_rtt_ms = Some(rtt_ms);
        self.last_published_at_ms = Some(now_ms);
    }

    pub(super) fn finish_attempt<E>(
        &mut self,
        rtt_ms: u32,
        now_ms: u64,
        result: Result<(), E>,
    ) -> Result<(), E> {
        if result.is_ok() {
            self.mark_published(rtt_ms, now_ms);
        }
        result
    }
}

#[cfg(test)]
mod tests;
