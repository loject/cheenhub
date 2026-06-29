//! Native-заглушка воспроизведения входящего аудио.
#![cfg_attr(
    any(
        target_arch = "wasm32",
        feature = "windows",
        feature = "linux",
        feature = "macos"
    ),
    allow(dead_code, unused_imports)
)]

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;

use super::backend::{NotificationSound, PlaybackCodec, VoiceFrame};
use super::output_devices::AudioOutputDevice;
use super::storage;

/// Контекстный хэндл воспроизведения аудио для native-клиента.
#[derive(Clone)]
pub(crate) struct AudioPlaybackHandle {
    muted: Signal<bool>,
    selected_output_device_id: Signal<Option<String>>,
    selected_output_device_label: Signal<Option<String>>,
    output_volume_percent: Signal<u32>,
    jitter_buffer_ms: Signal<u32>,
    warned_unsupported: Rc<Cell<bool>>,
}

impl AudioPlaybackHandle {
    /// Возвращает, отключено ли входящее воспроизведение.
    pub(crate) fn is_muted(&self) -> bool {
        (self.muted)()
    }

    /// Обновляет состояние отключения входящего воспроизведения.
    pub(crate) fn set_muted(&self, muted: bool) {
        if *self.muted.peek() == muted {
            return;
        }

        info!(muted, "native audio playback mute state changed");
        let mut muted_signal = self.muted;
        muted_signal.set(muted);
    }

    /// Запоминает громкость участника для UI без native playback backend.
    pub(crate) fn set_user_volume(&self, sender_user_id: &str, volume_percent: u32) {
        debug!(
            %sender_user_id,
            volume = volume_percent.min(200),
            "stored native participant audio volume without playback backend"
        );
    }

    /// Возвращает текущий процент общей громкости вывода.
    pub(crate) fn output_volume_percent(&self) -> u32 {
        (self.output_volume_percent)()
    }

    /// Обновляет процент общей громкости вывода.
    pub(crate) fn set_output_volume_percent(&self, volume_percent: u32) {
        let volume_percent = volume_percent.min(200);
        if *self.output_volume_percent.peek() == volume_percent {
            return;
        }

        info!(
            volume = volume_percent,
            "audio output volume preference changed"
        );
        storage::save_output_volume_percent(volume_percent);
        let mut volume_signal = self.output_volume_percent;
        volume_signal.set(volume_percent);
    }

    /// Возвращает текущую задержку jitter buffer для входящего голоса.
    pub(crate) fn jitter_buffer_ms(&self) -> u32 {
        (self.jitter_buffer_ms)()
    }

    /// Обновляет задержку jitter buffer для входящего голоса.
    pub(crate) fn set_jitter_buffer_ms(&self, buffer_ms: u32) {
        let buffer_ms = storage::clamp_jitter_buffer_ms(buffer_ms);
        if *self.jitter_buffer_ms.peek() == buffer_ms {
            return;
        }

        info!(buffer_ms, "inbound voice jitter buffer preference changed");
        storage::save_jitter_buffer_ms(buffer_ms);
        let mut jitter_buffer_signal = self.jitter_buffer_ms;
        jitter_buffer_signal.set(buffer_ms);
    }

    /// Сохраняет предпочитаемое устройство вывода.
    pub(crate) fn set_output_device(&self, device: &AudioOutputDevice) {
        self.set_output_device_preference(
            Some(device.device_id.clone()),
            Some(device.label.clone()),
        );
    }

    /// Сверяет сохраненное устройство вывода с перечисленными устройствами.
    pub(crate) fn reconcile_output_devices(&self, devices: &[AudioOutputDevice]) {
        let Some(selected_id) = self.selected_output_device_id.peek().clone() else {
            return;
        };
        if devices.iter().any(|device| device.device_id == selected_id) {
            return;
        }

        let Some(selected_label) = self.selected_output_device_label.peek().clone() else {
            return;
        };
        let Some(recovered) = devices
            .iter()
            .find(|device| !device.label.is_empty() && device.label == selected_label)
        else {
            return;
        };

        info!("recovered audio output device preference from stored label");
        self.set_output_device(recovered);
    }

    /// Возвращает текущий идентификатор предпочитаемого устройства вывода.
    pub(crate) fn output_device_id(&self) -> Option<String> {
        (self.selected_output_device_id)()
    }

    /// Останавливает состояние воспроизведения одного отправителя.
    #[allow(dead_code)]
    pub(crate) fn stop_sender(&self, _sender_user_id: &str) {}

    /// Останавливает все активное воспроизведение.
    pub(crate) fn stop_all(&self) {}

    /// На native пока только отмечает отсутствие backend'а воспроизведения.
    pub(crate) fn resume(&self) {
        if self.is_muted() {
            return;
        }
        self.warn_unsupported_once(None, None);
    }

    /// Принимает входящий voice frame без декодирования на native.
    pub(crate) fn play_voice_frame(&self, frame: VoiceFrame) {
        if frame.codec != PlaybackCodec::Opus || frame.bytes.is_empty() || self.is_muted() {
            return;
        }

        self.warn_unsupported_once(Some(&frame.sender_user_id), Some(frame.sequence));
    }

    /// Отмечает недоступность системных звуков без backend'а воспроизведения.
    pub(crate) fn play_notification_sound(&self, sound: NotificationSound) {
        if self.is_muted() {
            return;
        }

        debug!(
            sound = sound.event_name(),
            "skipped notification sound without native playback backend"
        );
    }

    fn set_output_device_preference(&self, device_id: Option<String>, label: Option<String>) {
        if self.selected_output_device_id.peek().as_deref() == device_id.as_deref()
            && self.selected_output_device_label.peek().as_deref() == label.as_deref()
        {
            return;
        }

        let next_has_device = device_id.as_ref().is_some_and(|id| !id.is_empty());
        info!(next_has_device, "audio output device preference changed");
        persist_output_device(device_id.as_deref(), label.as_deref());

        let mut id_signal = self.selected_output_device_id;
        let mut label_signal = self.selected_output_device_label;
        id_signal.set(device_id);
        label_signal.set(label);
    }

    fn warn_unsupported_once(&self, sender_user_id: Option<&str>, sequence: Option<u64>) {
        if self.warned_unsupported.replace(true) {
            return;
        }

        warn!(
            sender_user_id = sender_user_id.unwrap_or(""),
            sequence = sequence.unwrap_or_default(),
            "native audio playback backend is unavailable; inbound voice playback is disabled"
        );
    }
}

/// Предоставляет audio playback state компонентам аутентифицированного приложения.
#[component]
pub(crate) fn AudioPlaybackProvider(children: Element) -> Element {
    let muted = use_signal(|| false);
    let stored_output_device = storage::load_output_device();
    let output_volume_value = storage::load_output_volume_percent();
    let jitter_buffer_ms_value = storage::load_jitter_buffer_ms();
    let selected_output_device_id = use_signal({
        let stored_output_device = stored_output_device.clone();
        move || {
            stored_output_device
                .as_ref()
                .map(|device| device.device_id.clone())
        }
    });
    let selected_output_device_label =
        use_signal(move || stored_output_device.and_then(|device| device.label));
    let output_volume_percent = use_signal(move || output_volume_value);
    let jitter_buffer_ms = use_signal(move || jitter_buffer_ms_value);
    let handle = AudioPlaybackHandle {
        muted,
        selected_output_device_id,
        selected_output_device_label,
        output_volume_percent,
        jitter_buffer_ms,
        warned_unsupported: Rc::new(Cell::new(false)),
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}

fn persist_output_device(device_id: Option<&str>, label: Option<&str>) {
    match device_id {
        Some(device_id) if !device_id.is_empty() => {
            storage::save_output_device(device_id, label);
            info!(
                has_device = true,
                "persisted audio output device preference"
            );
        }
        _ => {
            storage::clear_output_device();
            info!(has_device = false, "cleared audio output device preference");
        }
    }
}
