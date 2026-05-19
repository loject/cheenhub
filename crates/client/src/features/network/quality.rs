//! Network quality state shared by network UI.

use dioxus::prelude::{Signal, WritableExt};

const HISTORY_WINDOW_MS: u64 = 60_000;

/// One measured realtime ping sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PingSample {
    /// Client timestamp when the pong arrived.
    pub(crate) received_at_ms: u64,
    /// Round-trip time in milliseconds.
    pub(crate) rtt_ms: f64,
}

/// Current network quality measurements.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NetworkQualityState {
    /// Latest measured round-trip time in milliseconds.
    pub(crate) latest_rtt_ms: Option<f64>,
    /// Ping samples retained for the latest minute.
    pub(crate) samples: Vec<PingSample>,
}

/// Cloneable network quality state handle.
#[derive(Clone, Copy)]
pub(crate) struct NetworkQualityHandle {
    state: Signal<NetworkQualityState>,
}

impl NetworkQualityHandle {
    /// Creates a handle backed by the provided Dioxus signal.
    pub(crate) fn new(state: Signal<NetworkQualityState>) -> Self {
        Self { state }
    }

    /// Returns the current network quality snapshot.
    pub(crate) fn current(&self) -> NetworkQualityState {
        (self.state)()
    }

    /// Records a completed ping-pong measurement.
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

    /// Clears quality data after the realtime session is lost.
    pub(crate) fn clear(&mut self) {
        self.state.set(NetworkQualityState::default());
    }
}
