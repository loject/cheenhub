//! Microphone context provider.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{
    MicrophoneBackend, MicrophoneCallbacks, MicrophoneConfig, MicrophoneFrameCallback,
    MicrophoneLevel, MicrophoneSession, MicrophoneStatus,
};
use super::browser::BrowserMicrophoneBackend;

const MICROPHONE_LEVEL_UPDATE_INTERVAL_US: u64 = 33_000;

/// Context handle used by features that need microphone input.
#[derive(Clone)]
pub(crate) struct MicrophoneHandle {
    status: Signal<MicrophoneStatus>,
    level: Signal<MicrophoneLevel>,
    session: Signal<Option<Rc<dyn MicrophoneSession>>>,
    generation: Signal<u64>,
    backend: Rc<dyn MicrophoneBackend>,
}

#[derive(Debug, Clone, Copy)]
struct LevelEmissionState {
    timestamp_us: u64,
    active: bool,
}

impl MicrophoneHandle {
    /// Starts microphone capture with the default configuration.
    pub(crate) fn start(&self, on_frame: MicrophoneFrameCallback) {
        if matches!(
            self.status(),
            MicrophoneStatus::Starting | MicrophoneStatus::Live
        ) {
            return;
        }

        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        let mut generation = self.generation;
        let start_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        reset_level(&mut level);

        spawn(async move {
            let callbacks = microphone_callbacks(on_frame, level);
            match backend.start(MicrophoneConfig::default(), callbacks).await {
                Ok(next_session) => {
                    if generation() != start_generation {
                        if let Err(error) = next_session.stop().await {
                            warn!(%error, "failed to stop stale microphone capture after start");
                        }
                        return;
                    }
                    session.set(Some(next_session));
                    status.set(MicrophoneStatus::Live);
                }
                Err(error) => {
                    if generation() != start_generation {
                        return;
                    }
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to start microphone capture");
                    session.set(None);
                    reset_level(&mut level);
                    status.set(next_status);
                }
            }
        });
    }

    /// Restarts microphone capture with a fresh frame callback.
    pub(crate) fn restart(&self, on_frame: MicrophoneFrameCallback) {
        let previous_session = self.session.peek().clone();
        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        let mut generation = self.generation;
        let restart_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        reset_level(&mut level);

        spawn(async move {
            if let Some(previous_session) = previous_session
                && let Err(error) = previous_session.stop().await
            {
                warn!(%error, "failed to stop previous microphone capture before restart");
            }
            if generation() != restart_generation {
                return;
            }

            let callbacks = microphone_callbacks(on_frame, level);
            match backend.start(MicrophoneConfig::default(), callbacks).await {
                Ok(next_session) => {
                    if generation() != restart_generation {
                        if let Err(error) = next_session.stop().await {
                            warn!(%error, "failed to stop stale microphone capture after restart");
                        }
                        return;
                    }
                    session.set(Some(next_session));
                    status.set(MicrophoneStatus::Live);
                }
                Err(error) => {
                    if generation() != restart_generation {
                        return;
                    }
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to restart microphone capture");
                    session.set(None);
                    reset_level(&mut level);
                    status.set(next_status);
                }
            }
        });
    }

    /// Stops the active microphone session.
    pub(crate) fn stop(&self) {
        let mut generation = self.generation;
        let stop_generation = next_generation(&mut generation);
        let Some(active_session) = self.session.peek().clone() else {
            let mut status = self.status;
            let mut level = self.level;
            reset_level(&mut level);
            status.set(MicrophoneStatus::Idle);
            return;
        };

        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        spawn(async move {
            if let Err(error) = active_session.stop().await {
                warn!(%error, "failed to stop microphone capture cleanly");
            }
            if generation() != stop_generation {
                return;
            }
            session.set(None);
            reset_level(&mut level);
            status.set(MicrophoneStatus::Idle);
        });
    }

    /// Toggles microphone capture.
    pub(crate) fn toggle(&self, on_frame: MicrophoneFrameCallback) {
        if matches!(
            self.status(),
            MicrophoneStatus::Live | MicrophoneStatus::Starting
        ) {
            self.stop();
        } else {
            self.start(on_frame);
        }
    }

    /// Returns the current microphone status.
    pub(crate) fn status(&self) -> MicrophoneStatus {
        (self.status)()
    }

    /// Returns the current microphone status without creating a reactive subscription.
    pub(crate) fn status_untracked(&self) -> MicrophoneStatus {
        self.status.peek().clone()
    }

    /// Returns the latest measured microphone input level.
    #[allow(dead_code)]
    pub(crate) fn level(&self) -> MicrophoneLevel {
        (self.level)()
    }

    /// Updates the active encoder bitrate.
    #[allow(dead_code)]
    pub(crate) fn set_bitrate_bps(&self, bitrate_bps: u32) {
        let Some(active_session) = (self.session)() else {
            return;
        };

        spawn(async move {
            if let Err(error) = active_session.set_bitrate_bps(bitrate_bps).await {
                warn!(%error, bitrate_bps, "failed to update microphone bitrate");
            }
        });
    }
}

fn microphone_callbacks(
    on_frame: MicrophoneFrameCallback,
    level: Signal<MicrophoneLevel>,
) -> MicrophoneCallbacks {
    let level = Rc::new(RefCell::new(level));
    let emission = Rc::new(RefCell::new(None::<LevelEmissionState>));
    MicrophoneCallbacks {
        on_frame,
        on_level: Rc::new(move |next_level| {
            if should_emit_level(&emission, next_level) {
                level.borrow_mut().set(next_level);
            }
        }),
    }
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

fn status_from_error(error: super::backend::MicrophoneError) -> MicrophoneStatus {
    if error.is_permission_denied() {
        MicrophoneStatus::PermissionDenied
    } else {
        MicrophoneStatus::Error(error.to_string())
    }
}

fn next_generation(generation: &mut Signal<u64>) -> u64 {
    let next_generation = generation.peek().saturating_add(1);
    generation.set(next_generation);
    next_generation
}

fn reset_level(level: &mut Signal<MicrophoneLevel>) {
    level.set(default_level());
}

fn default_level() -> MicrophoneLevel {
    MicrophoneLevel {
        rms: 0.0,
        active: false,
        threshold: MicrophoneConfig::default().vad_threshold,
        timestamp_us: 0,
    }
}

/// Provides microphone capture state to authenticated app components.
#[component]
pub(crate) fn MicrophoneProvider(children: Element) -> Element {
    let status = use_signal(|| MicrophoneStatus::Idle);
    let level = use_signal(default_level);
    let session = use_signal(|| None::<Rc<dyn MicrophoneSession>>);
    let generation = use_signal(|| 0);
    let backend: Rc<dyn MicrophoneBackend> = Rc::new(BrowserMicrophoneBackend);
    let handle = MicrophoneHandle {
        status,
        level,
        session,
        generation,
        backend,
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}
