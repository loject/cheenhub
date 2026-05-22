//! Microphone context provider.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{
    MicrophoneBackend, MicrophoneCallbacks, MicrophoneConfig, MicrophoneFrameCallback,
    MicrophoneLevel, MicrophoneSession, MicrophoneStatus,
};
use super::browser::BrowserMicrophoneBackend;
use super::storage;

const MICROPHONE_LEVEL_UPDATE_INTERVAL_US: u64 = 33_000;

/// Context handle used by features that need microphone input.
#[derive(Clone)]
pub(crate) struct MicrophoneHandle {
    status: Signal<MicrophoneStatus>,
    level: Signal<MicrophoneLevel>,
    session: Signal<Option<Rc<dyn MicrophoneSession>>>,
    generation: Signal<u64>,
    backend: Rc<dyn MicrophoneBackend>,
    selected_input_device_id: Signal<Option<String>>,
    /// Last on_frame callback used to start/restart capture.
    /// Kept so that device changes during an active session can trigger a restart.
    active_on_frame: Signal<Option<MicrophoneFrameCallback>>,
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
        let mut active_on_frame = self.active_on_frame;
        let device_id = self.selected_input_device_id.peek().clone();
        let start_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        active_on_frame.set(Some(on_frame.clone()));
        reset_level(&mut level);

        spawn(async move {
            let callbacks = microphone_callbacks(on_frame.clone(), level);
            let config = MicrophoneConfig {
                device_id,
                ..MicrophoneConfig::default()
            };
            match backend.start(config, callbacks).await {
                Ok(next_session) => {
                    if generation() != start_generation {
                        if let Err(error) = next_session.stop().await {
                            warn!(%error, "failed to stop stale microphone capture after start");
                        }
                        return;
                    }
                    session.set(Some(next_session));
                    status.set(MicrophoneStatus::Live);
                    active_on_frame.set(Some(on_frame));
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
                    active_on_frame.set(None);
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
        let mut active_on_frame = self.active_on_frame;
        let device_id = self.selected_input_device_id.peek().clone();
        let restart_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        active_on_frame.set(Some(on_frame.clone()));
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

            let callbacks = microphone_callbacks(on_frame.clone(), level);
            let config = MicrophoneConfig {
                device_id,
                ..MicrophoneConfig::default()
            };
            match backend.start(config, callbacks).await {
                Ok(next_session) => {
                    if generation() != restart_generation {
                        if let Err(error) = next_session.stop().await {
                            warn!(%error, "failed to stop stale microphone capture after restart");
                        }
                        return;
                    }
                    session.set(Some(next_session));
                    status.set(MicrophoneStatus::Live);
                    active_on_frame.set(Some(on_frame));
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
                    active_on_frame.set(None);
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
            let mut active_on_frame = self.active_on_frame;
            reset_level(&mut level);
            status.set(MicrophoneStatus::Idle);
            active_on_frame.set(None);
            return;
        };

        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        let mut active_on_frame = self.active_on_frame;
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
            active_on_frame.set(None);
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

    /// Stores the preferred input device ID.
    ///
    /// If the microphone is currently live the session is restarted on the new
    /// device immediately, keeping the same on_frame callback.
    pub(crate) fn set_input_device_id(&self, device_id: Option<String>) {
        if self.selected_input_device_id.peek().as_deref() == device_id.as_deref() {
            return;
        }
        let next_has_device = device_id.as_ref().is_some_and(|id| !id.is_empty());
        let status = self.status_untracked();
        debug!(
            ?status,
            next_has_device, "microphone input device preference changed"
        );
        persist_input_device_id(device_id.as_deref());
        let mut signal = self.selected_input_device_id;
        signal.set(device_id);

        let restart_on_frame =
            if matches!(status, MicrophoneStatus::Live | MicrophoneStatus::Starting) {
                self.active_on_frame.peek().clone()
            } else {
                None
            };

        if let Some(on_frame) = restart_on_frame {
            debug!(
                ?status,
                next_has_device, "restarting microphone capture after input device change"
            );
            self.restart(on_frame);
        }
    }

    /// Returns the currently preferred input device ID.
    pub(crate) fn input_device_id(&self) -> Option<String> {
        (self.selected_input_device_id)()
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
    let selected_input_device_id = use_signal(storage::load_input_device_id);
    let active_on_frame = use_signal(|| None::<MicrophoneFrameCallback>);
    let backend: Rc<dyn MicrophoneBackend> = Rc::new(BrowserMicrophoneBackend);
    let handle = MicrophoneHandle {
        status,
        level,
        session,
        generation,
        backend,
        selected_input_device_id,
        active_on_frame,
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}

fn persist_input_device_id(device_id: Option<&str>) {
    match device_id {
        Some(device_id) if !device_id.is_empty() => {
            storage::save_input_device_id(device_id);
            debug!(
                has_device = true,
                "persisted microphone input device preference"
            );
        }
        _ => {
            storage::clear_input_device_id();
            debug!(
                has_device = false,
                "cleared microphone input device preference"
            );
        }
    }
}
