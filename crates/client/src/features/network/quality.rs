//! Состояние качества сети, общее для сетевого UI.

use dioxus::prelude::{Signal, WritableExt, debug};

const HISTORY_WINDOW_MS: u64 = 60_000;

/// Один измеренный ping-сэмпл realtime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PingSample {
    /// Временная метка клиента, когда пришел pong.
    pub(crate) received_at_ms: u64,
    /// Время туда-обратно в миллисекундах.
    pub(crate) rtt_ms: f64,
    /// Сглаженное отклонение RTT от предыдущего измерения в миллисекундах.
    pub(crate) jitter_ms: f64,
}

/// Текущие измерения качества сети.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NetworkQualityState {
    /// Последнее измеренное время туда-обратно в миллисекундах.
    pub(crate) latest_rtt_ms: Option<f64>,
    /// Текущее сглаженное значение jitter в миллисекундах.
    pub(crate) latest_jitter_ms: Option<f64>,
    /// Ping-сэмплы, сохраненные за последнюю минуту.
    pub(crate) samples: Vec<PingSample>,
}

/// Клонируемый хендл состояния качества сети.
#[derive(Clone, Copy)]
pub(crate) struct NetworkQualityHandle {
    state: Signal<NetworkQualityState>,
}

impl NetworkQualityHandle {
    /// Создает хендл на базе переданного сигнала Dioxus.
    pub(crate) fn new(state: Signal<NetworkQualityState>) -> Self {
        Self { state }
    }

    /// Возвращает текущий снимок качества сети.
    pub(crate) fn current(&self) -> NetworkQualityState {
        (self.state)()
    }

    /// Записывает завершенное ping-pong-измерение.
    pub(crate) fn record_ping(&mut self, received_at_ms: u64, rtt_ms: f64) {
        self.state.with_mut(|state| {
            let jitter_ms = calculate_jitter(state.latest_rtt_ms, state.latest_jitter_ms, rtt_ms);
            debug!(
                rtt_ms,
                jitter_ms, "recorded realtime network quality sample"
            );
            state.latest_rtt_ms = Some(rtt_ms);
            state.latest_jitter_ms = Some(jitter_ms);
            state.samples.push(PingSample {
                received_at_ms,
                rtt_ms,
                jitter_ms,
            });
            let cutoff_ms = received_at_ms.saturating_sub(HISTORY_WINDOW_MS);
            state
                .samples
                .retain(|sample| sample.received_at_ms >= cutoff_ms);
        });
    }

    /// Очищает данные качества после потери realtime-сессии.
    pub(crate) fn clear(&mut self) {
        self.state.set(NetworkQualityState::default());
    }
}

fn calculate_jitter(
    previous_rtt_ms: Option<f64>,
    previous_jitter_ms: Option<f64>,
    rtt_ms: f64,
) -> f64 {
    match (previous_rtt_ms, previous_jitter_ms) {
        (Some(previous_rtt_ms), Some(previous_jitter_ms)) => {
            previous_jitter_ms * 15.0 / 16.0 + (rtt_ms - previous_rtt_ms).abs() / 16.0
        }
        (Some(previous_rtt_ms), None) => (rtt_ms - previous_rtt_ms).abs(),
        (None, _) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::calculate_jitter;

    #[test]
    fn smooths_rtt_changes_into_jitter() {
        assert_eq!(calculate_jitter(None, None, 100.0), 0.0);
        assert_eq!(calculate_jitter(Some(100.0), None, 140.0), 40.0);
        assert_eq!(calculate_jitter(Some(140.0), Some(40.0), 100.0), 40.0);
    }
}
