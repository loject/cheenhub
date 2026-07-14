//! Microphone context provider.

use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{
    MicrophoneActivationMode, MicrophoneBackend, MicrophoneConfig, MicrophoneFrameCallback,
    MicrophoneLevel, MicrophoneSession, MicrophoneStatus, MicrophoneUplinkConfig,
};
use super::provider_runtime::{
    gain_from_percent, microphone_callbacks, next_generation, reset_level, status_from_error,
    threshold_from_percent,
};

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
    /// Последняя uplink-конфигурация для restart активной сессии.
    pub(super) active_uplink: Signal<Option<MicrophoneUplinkConfig>>,
}

impl MicrophoneHandle {
    /// Starts microphone capture with the default configuration.
    #[allow(dead_code)]
    pub(crate) fn start(&self, on_frame: MicrophoneFrameCallback) {
        let active_capture = *self.active_capture.peek();
        match active_capture {
            ActiveCapture::Preview => {
                self.restart_capture(on_frame, ActiveCapture::Voice, None);
            }
            ActiveCapture::Voice
                if matches!(
                    self.status(),
                    MicrophoneStatus::Starting | MicrophoneStatus::Live
                ) => {}
            ActiveCapture::Voice => {
                self.restart_capture(on_frame, ActiveCapture::Voice, None);
            }
            _ => {
                self.start_capture(on_frame, ActiveCapture::Voice, None);
            }
        }
    }

    /// Запускает voice capture с отдельным low-latency uplink.
    #[allow(dead_code)]
    pub(crate) fn start_with_uplink(
        &self,
        on_frame: MicrophoneFrameCallback,
        uplink: MicrophoneUplinkConfig,
    ) {
        match *self.active_capture.peek() {
            ActiveCapture::Preview => {
                self.restart_capture(on_frame, ActiveCapture::Voice, Some(uplink));
            }
            ActiveCapture::Voice
                if matches!(
                    self.status(),
                    MicrophoneStatus::Starting | MicrophoneStatus::Live
                ) => {}
            ActiveCapture::Voice => {
                self.restart_capture(on_frame, ActiveCapture::Voice, Some(uplink));
            }
            _ => self.start_capture(on_frame, ActiveCapture::Voice, Some(uplink)),
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
        self.start_capture(Rc::new(|_| {}), ActiveCapture::Preview, None);
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

    fn start_capture(
        &self,
        on_frame: MicrophoneFrameCallback,
        capture: ActiveCapture,
        uplink: Option<MicrophoneUplinkConfig>,
    ) {
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
        let mut active_uplink = self.active_uplink;
        let device_id = self.selected_input_device_id.peek().clone();
        let input_gain = gain_from_percent(*self.input_volume_percent.peek());
        let activation_mode = *self.activation_mode.peek();
        let vad_threshold = threshold_from_percent(*self.vad_threshold_percent.peek());
        let start_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        active_capture.set(capture);
        active_on_frame.set(Some(on_frame.clone()));
        active_uplink.set(uplink.clone());
        let mut level_active = self.level_active;
        reset_level(&mut level, &mut level_active);

        spawn(async move {
            let callbacks = microphone_callbacks(
                on_frame.clone(),
                level,
                level_active,
                status,
                uplink.clone(),
            );
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
                    active_uplink.set(uplink);
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
                    active_uplink.set(None);
                }
            }
        });
    }

    /// Restarts microphone capture with a fresh frame callback.
    #[allow(dead_code)]
    pub(crate) fn restart(&self, on_frame: MicrophoneFrameCallback) {
        self.restart_capture(on_frame, ActiveCapture::Voice, None);
    }

    /// Перезапускает voice capture с отдельным low-latency uplink.
    #[allow(dead_code)]
    pub(crate) fn restart_with_uplink(
        &self,
        on_frame: MicrophoneFrameCallback,
        uplink: MicrophoneUplinkConfig,
    ) {
        self.restart_capture(on_frame, ActiveCapture::Voice, Some(uplink));
    }

    pub(super) fn restart_capture(
        &self,
        on_frame: MicrophoneFrameCallback,
        capture: ActiveCapture,
        uplink: Option<MicrophoneUplinkConfig>,
    ) {
        let previous_session = self.session.peek().clone();
        let backend = self.backend.clone();
        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        let mut generation = self.generation;
        let mut active_capture = self.active_capture;
        let mut active_on_frame = self.active_on_frame;
        let mut active_uplink = self.active_uplink;
        let device_id = self.selected_input_device_id.peek().clone();
        let input_gain = gain_from_percent(*self.input_volume_percent.peek());
        let activation_mode = *self.activation_mode.peek();
        let vad_threshold = threshold_from_percent(*self.vad_threshold_percent.peek());
        let restart_generation = next_generation(&mut generation);
        status.set(MicrophoneStatus::Starting);
        active_capture.set(capture);
        active_on_frame.set(Some(on_frame.clone()));
        active_uplink.set(uplink.clone());
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

            let callbacks = microphone_callbacks(
                on_frame.clone(),
                level,
                level_active,
                status,
                uplink.clone(),
            );
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
                    active_uplink.set(uplink);
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
                    active_uplink.set(None);
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
            let mut active_uplink = self.active_uplink;
            reset_level(&mut level, &mut level_active);
            status.set(MicrophoneStatus::Idle);
            active_capture.set(ActiveCapture::None);
            active_on_frame.set(None);
            active_uplink.set(None);
            return;
        };

        let mut session = self.session;
        let mut status = self.status;
        let mut level = self.level;
        let mut level_active = self.level_active;
        let mut active_capture = self.active_capture;
        let mut active_on_frame = self.active_on_frame;
        let mut active_uplink = self.active_uplink;
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
            active_uplink.set(None);
        });
    }

    /// Toggles microphone capture.
    #[allow(dead_code)]
    pub(crate) fn toggle(&self, on_frame: MicrophoneFrameCallback) {
        let active_capture = *self.active_capture.peek();
        match active_capture {
            ActiveCapture::Preview => {
                self.restart_capture(on_frame, ActiveCapture::Voice, None);
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

    /// Возвращает поколение текущей операции для отмены устаревшей внешней подготовки.
    #[allow(dead_code)]
    pub(crate) fn operation_generation(&self) -> u64 {
        *self.generation.peek()
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
