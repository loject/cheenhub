//! Microphone context provider.

use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{
    MicrophoneActivationMode, MicrophoneBackend, MicrophoneConfig, MicrophoneFrameCallback,
    MicrophoneLevel, MicrophoneSession, MicrophoneStatus,
};
use super::input_devices::AudioInputDevice;
use super::provider_runtime::{
    gain_from_percent, microphone_callbacks, next_generation, persist_input_device, reset_level,
    status_from_error, threshold_from_percent,
};
use super::storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActiveCapture {
    None,
    Preview,
    Voice,
}

/// Context handle used by features that need microphone input.
#[derive(Clone)]
pub(crate) struct MicrophoneHandle {
    pub(super) status: Signal<MicrophoneStatus>,
    pub(super) level: Signal<MicrophoneLevel>,
    pub(super) level_active: Signal<bool>,
    pub(super) session: Signal<Option<Rc<dyn MicrophoneSession>>>,
    pub(super) generation: Signal<u64>,
    pub(super) backend: Rc<dyn MicrophoneBackend>,
    pub(super) selected_input_device_id: Signal<Option<String>>,
    pub(super) selected_input_device_label: Signal<Option<String>>,
    pub(super) input_volume_percent: Signal<u32>,
    pub(super) activation_mode: Signal<MicrophoneActivationMode>,
    pub(super) vad_threshold_percent: Signal<u32>,
    pub(super) active_capture: Signal<ActiveCapture>,
    /// Last on_frame callback used to start/restart capture.
    /// Kept so that device changes during an active session can trigger a restart.
    pub(super) active_on_frame: Signal<Option<MicrophoneFrameCallback>>,
}

impl MicrophoneHandle {
    /// Starts microphone capture with the default configuration.
    pub(crate) fn start(&self, on_frame: MicrophoneFrameCallback) {
        let active_capture = *self.active_capture.peek();
        match active_capture {
            ActiveCapture::Preview => {
                self.restart_capture(on_frame, ActiveCapture::Voice);
            }
            ActiveCapture::Voice
                if matches!(
                    self.status(),
                    MicrophoneStatus::Starting | MicrophoneStatus::Live
                ) => {}
            _ => {
                self.start_capture(on_frame, ActiveCapture::Voice);
            }
        }
    }

    /// Starts microphone capture for settings level preview when no voice capture is active.
    pub(crate) fn start_level_preview(&self) {
        let active_capture = *self.active_capture.peek();
        if !matches!(self.status_untracked(), MicrophoneStatus::Idle)
            || active_capture != ActiveCapture::None
        {
            return;
        }

        info!("starting microphone level preview capture");
        self.start_capture(Rc::new(|_| {}), ActiveCapture::Preview);
    }

    /// Stops microphone capture only when it is owned by the settings level preview.
    pub(crate) fn stop_level_preview(&self) {
        let active_capture = *self.active_capture.peek();
        if active_capture != ActiveCapture::Preview {
            return;
        }

        info!("stopping microphone level preview capture");
        self.stop();
    }

    fn start_capture(&self, on_frame: MicrophoneFrameCallback, capture: ActiveCapture) {
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
        let mut active_capture = self.active_capture;
        let mut active_on_frame = self.active_on_frame;
        let device_id = self.selected_input_device_id.peek().clone();
        let input_gain = gain_from_percent(*self.input_volume_percent.peek());
        let activation_mode = *self.activation_mode.peek();
        let vad_threshold = threshold_from_percent(*self.vad_threshold_percent.peek());
        let start_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        active_capture.set(capture);
        active_on_frame.set(Some(on_frame.clone()));
        let mut level_active = self.level_active;
        reset_level(&mut level, &mut level_active);

        spawn(async move {
            let callbacks = microphone_callbacks(on_frame.clone(), level, level_active);
            let config = MicrophoneConfig {
                device_id,
                input_gain,
                activation_mode,
                vad_threshold,
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
                    active_capture.set(capture);
                    active_on_frame.set(Some(on_frame));
                }
                Err(error) => {
                    if generation() != start_generation {
                        return;
                    }
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to start microphone capture");
                    session.set(None);
                    reset_level(&mut level, &mut level_active);
                    status.set(next_status);
                    active_capture.set(ActiveCapture::None);
                    active_on_frame.set(None);
                }
            }
        });
    }

    /// Restarts microphone capture with a fresh frame callback.
    pub(crate) fn restart(&self, on_frame: MicrophoneFrameCallback) {
        self.restart_capture(on_frame, ActiveCapture::Voice);
    }

    fn restart_capture(&self, on_frame: MicrophoneFrameCallback, capture: ActiveCapture) {
        let previous_session = self.session.peek().clone();
        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        let mut generation = self.generation;
        let mut active_capture = self.active_capture;
        let mut active_on_frame = self.active_on_frame;
        let device_id = self.selected_input_device_id.peek().clone();
        let input_gain = gain_from_percent(*self.input_volume_percent.peek());
        let activation_mode = *self.activation_mode.peek();
        let vad_threshold = threshold_from_percent(*self.vad_threshold_percent.peek());
        let restart_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        active_capture.set(capture);
        active_on_frame.set(Some(on_frame.clone()));
        let mut level_active = self.level_active;
        reset_level(&mut level, &mut level_active);

        spawn(async move {
            if let Some(previous_session) = previous_session
                && let Err(error) = previous_session.stop().await
            {
                warn!(%error, "failed to stop previous microphone capture before restart");
            }
            if generation() != restart_generation {
                return;
            }

            let callbacks = microphone_callbacks(on_frame.clone(), level, level_active);
            let config = MicrophoneConfig {
                device_id,
                input_gain,
                activation_mode,
                vad_threshold,
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
                    active_capture.set(capture);
                    active_on_frame.set(Some(on_frame));
                }
                Err(error) => {
                    if generation() != restart_generation {
                        return;
                    }
                    let next_status = status_from_error(error.clone());
                    warn!(%error, status = ?next_status, "failed to restart microphone capture");
                    session.set(None);
                    reset_level(&mut level, &mut level_active);
                    status.set(next_status);
                    active_capture.set(ActiveCapture::None);
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
            let mut level_active = self.level_active;
            let mut active_capture = self.active_capture;
            let mut active_on_frame = self.active_on_frame;
            reset_level(&mut level, &mut level_active);
            status.set(MicrophoneStatus::Idle);
            active_capture.set(ActiveCapture::None);
            active_on_frame.set(None);
            return;
        };

        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        let mut level_active = self.level_active;
        let mut active_capture = self.active_capture;
        let mut active_on_frame = self.active_on_frame;
        spawn(async move {
            if let Err(error) = active_session.stop().await {
                warn!(%error, "failed to stop microphone capture cleanly");
            }
            if generation() != stop_generation {
                return;
            }
            session.set(None);
            reset_level(&mut level, &mut level_active);
            status.set(MicrophoneStatus::Idle);
            active_capture.set(ActiveCapture::None);
            active_on_frame.set(None);
        });
    }

    /// Toggles microphone capture.
    pub(crate) fn toggle(&self, on_frame: MicrophoneFrameCallback) {
        let active_capture = *self.active_capture.peek();
        match active_capture {
            ActiveCapture::Preview => {
                self.restart_capture(on_frame, ActiveCapture::Voice);
            }
            ActiveCapture::Voice
                if matches!(
                    self.status(),
                    MicrophoneStatus::Live | MicrophoneStatus::Starting
                ) =>
            {
                self.stop();
            }
            _ => {
                self.start(on_frame);
            }
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

    /// Возвращает, открыт ли gate голосовой активности микрофона.
    pub(crate) fn level_active(&self) -> bool {
        (self.level_active)()
    }

    /// Сохраняет preference устройства ввода; пустой device_id означает системное устройство по умолчанию.
    pub(crate) fn set_input_device(&self, device: &AudioInputDevice) {
        self.set_input_device_preference(
            Some(device.device_id.clone()),
            Some(device.label.clone()),
        );
    }

    /// Reconciles a stored device preference against the currently enumerated devices.
    pub(crate) fn reconcile_input_devices(&self, devices: &[AudioInputDevice]) {
        let Some(selected_id) = self.selected_input_device_id.peek().clone() else {
            return;
        };
        if devices.iter().any(|device| device.device_id == selected_id) {
            return;
        }

        let Some(selected_label) = self.selected_input_device_label.peek().clone() else {
            return;
        };
        let Some(recovered) = devices
            .iter()
            .find(|device| !device.label.is_empty() && device.label == selected_label)
        else {
            return;
        };

        info!("recovered microphone input device preference from stored label");
        self.set_input_device(recovered);
    }

    fn set_input_device_preference(&self, device_id: Option<String>, label: Option<String>) {
        let device_id = device_id.filter(|id| !id.is_empty());
        let label = device_id.as_ref().and(label);
        if self.selected_input_device_id.peek().as_deref() == device_id.as_deref()
            && self.selected_input_device_label.peek().as_deref() == label.as_deref()
        {
            return;
        }

        let next_has_device = device_id.as_ref().is_some_and(|id| !id.is_empty());
        let status = self.status_untracked();
        info!(
            ?status,
            next_has_device, "microphone input device preference changed"
        );
        persist_input_device(device_id.as_deref(), label.as_deref());
        let mut id_signal = self.selected_input_device_id;
        let mut label_signal = self.selected_input_device_label;
        id_signal.set(device_id);
        label_signal.set(label);

        let restart_on_frame =
            if matches!(status, MicrophoneStatus::Live | MicrophoneStatus::Starting) {
                self.active_on_frame.peek().clone()
            } else {
                None
            };

        if let Some(on_frame) = restart_on_frame {
            let active_capture = *self.active_capture.peek();
            info!(
                ?status,
                next_has_device, "restarting microphone capture after input device change"
            );
            self.restart_capture(on_frame, active_capture);
        }
    }

    /// Returns the currently preferred input device ID.
    pub(crate) fn input_device_id(&self) -> Option<String> {
        (self.selected_input_device_id)()
    }

    /// Returns the current microphone input volume percentage.
    pub(crate) fn input_volume_percent(&self) -> u32 {
        (self.input_volume_percent)()
    }

    /// Updates the microphone input volume percentage.
    pub(crate) fn set_input_volume_percent(&self, volume_percent: u32) {
        let volume_percent = volume_percent.min(200);
        if *self.input_volume_percent.peek() == volume_percent {
            return;
        }

        let status = self.status_untracked();
        info!(
            ?status,
            volume = volume_percent,
            "microphone input volume preference changed"
        );
        storage::save_input_volume_percent(volume_percent);
        let mut input_volume = self.input_volume_percent;
        input_volume.set(volume_percent);

        let restart_on_frame =
            if matches!(status, MicrophoneStatus::Live | MicrophoneStatus::Starting) {
                self.active_on_frame.peek().clone()
            } else {
                None
            };
        if let Some(on_frame) = restart_on_frame {
            let active_capture = *self.active_capture.peek();
            info!(
                ?status,
                "restarting microphone capture after input volume change"
            );
            self.restart_capture(on_frame, active_capture);
        }
    }

    /// Returns the current microphone activation mode.
    pub(crate) fn activation_mode(&self) -> MicrophoneActivationMode {
        (self.activation_mode)()
    }

    /// Updates the microphone activation mode.
    pub(crate) fn set_activation_mode(&self, mode: MicrophoneActivationMode) {
        if *self.activation_mode.peek() == mode {
            return;
        }

        let status = self.status_untracked();
        info!(
            ?status,
            ?mode,
            "microphone activation mode preference changed"
        );
        storage::save_activation_mode(mode);
        let mut activation_mode = self.activation_mode;
        activation_mode.set(mode);
        self.restart_if_active(status, "microphone activation mode change");
    }

    /// Returns the current voice activation threshold percentage.
    pub(crate) fn vad_threshold_percent(&self) -> u32 {
        (self.vad_threshold_percent)()
    }

    /// Updates the voice activation threshold percentage.
    pub(crate) fn set_vad_threshold_percent(&self, threshold_percent: u32) {
        let threshold_percent = threshold_percent.min(100);
        if *self.vad_threshold_percent.peek() == threshold_percent {
            return;
        }

        let status = self.status_untracked();
        info!(
            ?status,
            threshold = threshold_percent,
            "microphone vad threshold preference changed"
        );
        storage::save_vad_threshold_percent(threshold_percent);
        let mut vad_threshold = self.vad_threshold_percent;
        vad_threshold.set(threshold_percent);
        self.restart_if_active(status, "microphone vad threshold change");
    }

    fn restart_if_active(&self, status: MicrophoneStatus, reason: &'static str) {
        let restart_on_frame =
            if matches!(status, MicrophoneStatus::Live | MicrophoneStatus::Starting) {
                self.active_on_frame.peek().clone()
            } else {
                None
            };
        if let Some(on_frame) = restart_on_frame {
            let active_capture = *self.active_capture.peek();
            info!(?status, reason, "restarting microphone capture");
            self.restart_capture(on_frame, active_capture);
        }
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
