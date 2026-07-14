//! Настройки активного microphone provider и их применение к capture-сессии.

use dioxus::prelude::*;

use super::backend::{MicrophoneActivationMode, MicrophoneStatus};
use super::input_devices::AudioInputDevice;
use super::provider::MicrophoneHandle;
use super::{provider_runtime::persist_input_device, storage};

impl MicrophoneHandle {
    /// Сохраняет устройство ввода; пустой device_id означает системное устройство.
    pub(crate) fn set_input_device(&self, device: &AudioInputDevice) {
        self.set_input_device_preference(
            Some(device.device_id.clone()),
            Some(device.label.clone()),
        );
    }

    /// Сопоставляет сохранённое устройство с актуальным списком по метке.
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

        if let Some(on_frame) = self.restart_callback(status.clone()) {
            let active_capture = *self.active_capture.peek();
            info!(
                ?status,
                next_has_device, "restarting microphone capture after input device change"
            );
            let uplink = self.active_uplink.peek().clone();
            self.restart_capture(on_frame, active_capture, uplink);
        }
    }

    /// Возвращает выбранный идентификатор устройства ввода.
    pub(crate) fn input_device_id(&self) -> Option<String> {
        (self.selected_input_device_id)()
    }

    /// Возвращает громкость входа в процентах.
    pub(crate) fn input_volume_percent(&self) -> u32 {
        (self.input_volume_percent)()
    }

    /// Обновляет громкость входа в процентах.
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

        if let Some(on_frame) = self.restart_callback(status.clone()) {
            let active_capture = *self.active_capture.peek();
            info!(
                ?status,
                "restarting microphone capture after input volume change"
            );
            let uplink = self.active_uplink.peek().clone();
            self.restart_capture(on_frame, active_capture, uplink);
        }
    }

    /// Возвращает режим активации микрофона.
    pub(crate) fn activation_mode(&self) -> MicrophoneActivationMode {
        (self.activation_mode)()
    }

    /// Обновляет режим активации микрофона.
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

    /// Возвращает порог voice activation в процентах.
    pub(crate) fn vad_threshold_percent(&self) -> u32 {
        (self.vad_threshold_percent)()
    }

    /// Обновляет порог voice activation в процентах.
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

    fn restart_callback(
        &self,
        status: MicrophoneStatus,
    ) -> Option<super::backend::MicrophoneFrameCallback> {
        matches!(status, MicrophoneStatus::Live | MicrophoneStatus::Starting)
            .then(|| self.active_on_frame.peek().clone())
            .flatten()
    }

    fn restart_if_active(&self, status: MicrophoneStatus, reason: &'static str) {
        if let Some(on_frame) = self.restart_callback(status.clone()) {
            let active_capture = *self.active_capture.peek();
            info!(?status, reason, "restarting microphone capture");
            let uplink = self.active_uplink.peek().clone();
            self.restart_capture(on_frame, active_capture, uplink);
        }
    }
}
