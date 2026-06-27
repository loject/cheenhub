//! Microphone provider runtime helpers.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;

use super::backend::{
    EncodedMicrophoneFrame, MicrophoneConfig, MicrophoneError, MicrophoneFrameCallback,
    MicrophoneLevel, MicrophoneStatus,
};

const MICROPHONE_LEVEL_UPDATE_INTERVAL_US: u64 = 33_000;

#[derive(Debug, Clone, Copy)]
struct LevelEmissionState {
    timestamp_us: u64,
    active: bool,
}

pub(super) fn microphone_callbacks(
    on_frame: MicrophoneFrameCallback,
    level: Signal<MicrophoneLevel>,
) -> super::backend::MicrophoneCallbacks {
    let (events, receiver) = mpsc::unbounded();
    spawn_microphone_callback_relay(receiver, on_frame, level);

    let frame_events = events.clone();
    super::backend::MicrophoneCallbacks {
        on_frame: Rc::new(move |frame| {
            let _ = frame_events.unbounded_send(MicrophoneCallbackEvent::Frame(frame));
        }),
        on_level: Rc::new(move |next_level| {
            let _ = events.unbounded_send(MicrophoneCallbackEvent::Level(next_level));
        }),
    }
}

fn spawn_microphone_callback_relay(
    mut receiver: mpsc::UnboundedReceiver<MicrophoneCallbackEvent>,
    on_frame: MicrophoneFrameCallback,
    mut level: Signal<MicrophoneLevel>,
) {
    spawn(async move {
        let emission = Rc::new(RefCell::new(None::<LevelEmissionState>));
        while let Some(event) = receiver.next().await {
            match event {
                MicrophoneCallbackEvent::Frame(frame) => on_frame(frame),
                MicrophoneCallbackEvent::Level(next_level) => {
                    if should_emit_level(&emission, next_level) {
                        level.set(next_level);
                    }
                }
            }
        }
        debug!("microphone callback relay stopped");
    });
}

enum MicrophoneCallbackEvent {
    Frame(EncodedMicrophoneFrame),
    Level(MicrophoneLevel),
}

pub(super) fn status_from_error(error: MicrophoneError) -> MicrophoneStatus {
    if error.is_permission_denied() {
        MicrophoneStatus::PermissionDenied
    } else {
        MicrophoneStatus::Error(error.to_string())
    }
}

pub(super) fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}

pub(super) fn reset_level(level: &mut Signal<MicrophoneLevel>) {
    level.set(default_level());
}

pub(super) fn default_level() -> MicrophoneLevel {
    MicrophoneLevel {
        rms: 0.0,
        active: false,
        threshold: MicrophoneConfig::default().vad_threshold,
        timestamp_us: 0,
    }
}

pub(super) fn gain_from_percent(volume_percent: u32) -> f32 {
    volume_percent.min(200) as f32 / 100.0
}

pub(super) fn threshold_from_percent(threshold_percent: u32) -> f32 {
    threshold_percent.min(100) as f32 / 1000.0
}

fn should_emit_level(
    emission: &Rc<RefCell<Option<LevelEmissionState>>>,
    next_level: MicrophoneLevel,
) -> bool {
    let mut emission = emission.borrow_mut();
    let Some(previous) = *emission else {
        *emission = Some(LevelEmissionState {
            timestamp_us: next_level.timestamp_us,
            active: next_level.active,
        });
        return true;
    };

    let active_changed = previous.active != next_level.active;
    let interval_elapsed = next_level.timestamp_us > previous.timestamp_us
        && next_level
            .timestamp_us
            .saturating_sub(previous.timestamp_us)
            >= MICROPHONE_LEVEL_UPDATE_INTERVAL_US;
    if !active_changed && !interval_elapsed {
        return false;
    }

    *emission = Some(LevelEmissionState {
        timestamp_us: next_level.timestamp_us,
        active: next_level.active,
    });
    true
}
