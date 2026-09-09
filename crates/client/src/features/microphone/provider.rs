//! Microphone context provider.

use std::rc::Rc;

use dioxus::prelude::*;
use futures_channel::mpsc;

use super::backend::{
    MicrophoneActivationMode, MicrophoneFrameCallback, MicrophoneLevel, MicrophoneStatus,
    MicrophoneUplinkConfig,
};
use super::provider_runtime::next_generation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActiveCapture {
    None,
    Preview,
    Voice,
}

pub(super) enum MicrophoneCommand {
    Start {
        operation_generation: u64,
        on_frame: MicrophoneFrameCallback,
        capture: ActiveCapture,
        uplink: Option<MicrophoneUplinkConfig>,
    },
    Restart {
        operation_generation: u64,
        on_frame: MicrophoneFrameCallback,
        capture: ActiveCapture,
        uplink: Option<MicrophoneUplinkConfig>,
    },
    Stop {
        operation_generation: u64,
    },
    SetBitrate {
        bitrate_bps: u32,
    },
}

/// Context handle used by features that need microphone input.
#[derive(Clone)]
pub(crate) struct MicrophoneHandle {
    pub(super) status: Signal<MicrophoneStatus>,
    pub(super) level: Signal<MicrophoneLevel>,
    pub(super) level_active: Signal<bool>,
    pub(super) generation: Signal<u64>,
    pub(super) commands: mpsc::UnboundedSender<MicrophoneCommand>,
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
    /// Целевой битрейт кодировщика; применяется при старте и live-сессии.
    pub(super) target_bitrate_bps: Signal<u32>,
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
                    self.status_untracked(),
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
                    self.status_untracked(),
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
        let status = self.status_untracked();
        if !should_start_level_preview(&status, active_capture) {
            debug!(
                ?status,
                ?active_capture,
                "skipping microphone level preview capture start"
            );
            return;
        }

        info!(
            previous_status = ?status,
            ?active_capture,
            "starting microphone level preview capture"
        );
        self.start_capture(Rc::new(|_| {}), ActiveCapture::Preview, None);
    }

    /// Stops microphone capture only when it is owned by the settings level preview.
    pub(crate) fn stop_level_preview(&self) {
        let active_capture = *self.active_capture.peek();
        if !should_stop_level_preview(active_capture) {
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
        // Lifecycle-команды могут продолжаться после `await` вне реактивного scope Dioxus.
        // Здесь нужна только моментальная проверка, а не подписка текущей задачи на Signal.
        let previous_status = self.status_untracked();
        if matches!(
            previous_status,
            MicrophoneStatus::Starting | MicrophoneStatus::Live
        ) {
            debug!(
                ?previous_status,
                ?capture,
                "skipping duplicate microphone capture start"
            );
            return;
        }

        let mut generation = self.generation;
        let operation_generation = next_generation(&mut generation);
        self.send_command(MicrophoneCommand::Start {
            operation_generation,
            on_frame,
            capture,
            uplink,
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
        let mut generation = self.generation;
        let operation_generation = next_generation(&mut generation);
        self.send_command(MicrophoneCommand::Restart {
            operation_generation,
            on_frame,
            capture,
            uplink,
        });
    }

    /// Stops the active microphone session.
    pub(crate) fn stop(&self) {
        let mut generation = self.generation;
        let operation_generation = next_generation(&mut generation);
        self.send_command(MicrophoneCommand::Stop {
            operation_generation,
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
                    self.status_untracked(),
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

    /// Updates the target encoder bitrate for current and future sessions.
    pub(crate) fn set_bitrate_bps(&self, bitrate_bps: u32) {
        let mut target_bitrate = self.target_bitrate_bps;
        if *target_bitrate.peek() == bitrate_bps {
            return;
        }
        target_bitrate.set(bitrate_bps);
        self.send_command(MicrophoneCommand::SetBitrate { bitrate_bps });
    }

    fn send_command(&self, command: MicrophoneCommand) {
        if self.commands.unbounded_send(command).is_err() {
            warn!("microphone runtime command receiver is unavailable");
        }
    }
}

fn should_start_level_preview(status: &MicrophoneStatus, active_capture: ActiveCapture) -> bool {
    active_capture == ActiveCapture::None
        && matches!(
            status,
            MicrophoneStatus::Idle
                | MicrophoneStatus::PermissionDenied
                | MicrophoneStatus::Error(_)
        )
}

fn should_stop_level_preview(active_capture: ActiveCapture) -> bool {
    active_capture == ActiveCapture::Preview
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
