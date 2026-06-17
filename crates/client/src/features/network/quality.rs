//! Состояние качества сети, общее для сетевого UI.

use dioxus::prelude::{Signal, WritableExt};

const HISTORY_WINDOW_MS: u64 = 60_000;

/// Один измеренный ping-сэмпл realtime.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PingSample {
    /// Временная метка клиента, когда пришел pong.
    pub(crate) received_at_ms: u64,
    /// Время туда-обратно в миллисекундах.
    pub(crate) rtt_ms: f64,
}

/// Текущие измерения качества сети.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NetworkQualityState {
    /// Последнее измеренное время туда-обратно в миллисекундах.
    pub(crate) latest_rtt_ms: Option<f64>,
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
            state.latest_rtt_ms = Some(rtt_ms);
            state.samples.push(PingSample {
                received_at_ms,
                rtt_ms,
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
