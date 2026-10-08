//! Мост событий активной microphone-сессии к состоянию провайдера.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::microphone::backend::{
    EncodedMicrophoneFrame, MicrophoneError, MicrophoneFrameCallback, MicrophoneLevel,
    MicrophoneStatus,
};
use crate::features::toast::ToastHandle;

const MICROPHONE_LEVEL_UPDATE_INTERVAL_US: u64 = 33_000;

#[derive(Debug, Clone, Copy)]
struct LevelEmissionState {
    timestamp_us: u64,
    active: bool,
}

pub(in crate::features::microphone) fn microphone_callbacks(
    on_frame: MicrophoneFrameCallback,
    level: Signal<MicrophoneLevel>,
    level_active: Signal<bool>,
    status: Signal<MicrophoneStatus>,
    uplink: Option<crate::features::microphone::backend::MicrophoneUplinkConfig>,
    toast: ToastHandle,
) -> crate::features::microphone::backend::MicrophoneCallbacks {
    let (events, receiver) = mpsc::unbounded();
    spawn_microphone_callback_relay(receiver, on_frame, level, level_active, status, toast);
    let frame_events = events.clone();
    let error_events = events.clone();
    let warning_events = events.clone();
    crate::features::microphone::backend::MicrophoneCallbacks {
        on_frame: Rc::new(move |frame| {
            let _ = frame_events.unbounded_send(MicrophoneCallbackEvent::Frame(frame));
        }),
        on_level: Rc::new(move |next_level| {
            let _ = events.unbounded_send(MicrophoneCallbackEvent::Level(next_level));
        }),
        on_error: Rc::new(move |error| {
            let _ = error_events.unbounded_send(MicrophoneCallbackEvent::Error(error));
        }),
        on_warning: Rc::new(move |message| {
            let _ = warning_events.unbounded_send(MicrophoneCallbackEvent::Warning(message));
        }),
        uplink,
    }
}

fn spawn_microphone_callback_relay(
    mut receiver: mpsc::UnboundedReceiver<MicrophoneCallbackEvent>,
    on_frame: MicrophoneFrameCallback,
    mut level: Signal<MicrophoneLevel>,
    mut level_active: Signal<bool>,
    mut status: Signal<MicrophoneStatus>,
    toast: ToastHandle,
) {
    spawn(async move {
        let emission = Rc::new(RefCell::new(None::<LevelEmissionState>));
        while let Some(event) = receiver.next().await {
            match event {
                MicrophoneCallbackEvent::Frame(frame) => on_frame(frame),
                MicrophoneCallbackEvent::Level(next_level) => {
                    if should_emit_level(&emission, next_level) {
                        if *level_active.peek() != next_level.active {
                            level_active.set(next_level.active);
                        }
                        level.set(next_level);
                    }
                }
                MicrophoneCallbackEvent::Error(error) => {
                    warn!(%error, "active microphone backend failed");
                    status.set(status_from_error(error));
                    reset_level(&mut level, &mut level_active);
                }
                MicrophoneCallbackEvent::Warning(message) => {
                    warn!(
                        kind = "denoise_failed",
                        "microphone DSP degraded; capture remains active"
                    );
                    apply_warning(message, &toast);
                }
            }
        }
        debug!("microphone callback relay stopped");
    });
}

enum MicrophoneCallbackEvent {
    Frame(EncodedMicrophoneFrame),
    Level(MicrophoneLevel),
    Error(MicrophoneError),
    Warning(&'static str),
}

fn apply_warning(message: &'static str, toast: &ToastHandle) {
    toast.warning(message);
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

fn reset_level(level: &mut Signal<MicrophoneLevel>, active: &mut Signal<bool>) {
    if *active.peek() {
        active.set(false);
    }
    level.set(super::default_level());
}

pub(in crate::features::microphone) fn status_from_error(
    error: MicrophoneError,
) -> MicrophoneStatus {
    if error.is_permission_denied() {
        MicrophoneStatus::PermissionDenied
    } else {
        MicrophoneStatus::Error(error.to_string())
    }
}

#[cfg(test)]
mod tests;
