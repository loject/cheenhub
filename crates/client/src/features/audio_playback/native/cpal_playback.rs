//! Native-провайдер воспроизведения аудио через `cpal`.

mod engine;
#[path = "../jitter_buffer.rs"]
mod jitter_buffer;
mod jitter_runtime;
mod mixer;
mod notification_sounds;
mod output_samples;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use opus::Decoder;
use web_time::{SystemTime, UNIX_EPOCH};

use self::engine::{NativePlaybackEngine, create_engine};
use self::jitter_buffer::JitterBuffer;
use self::mixer::{
    clear_mixer, clear_voice_senders, remove_sender, update_output_gain, update_sender_gain,
};
use crate::features::audio_playback::output_devices::AudioOutputDevice;
use crate::features::audio_playback::storage;

const AUDIO_SAMPLE_RATE_HZ: u32 = 48_000;

/// Контекстный хэндл native-аудиовоспроизведения.
#[derive(Clone)]
pub(crate) struct AudioPlaybackHandle {
    muted: Signal<bool>,
    selected_output_device_id: Signal<Option<String>>,
    selected_output_device_label: Signal<Option<String>>,
    output_volume_percent: Signal<u32>,
    jitter_buffer_us: Signal<u32>,
    inner: Rc<RefCell<AudioPlaybackInner>>,
}

struct AudioPlaybackInner {
    muted: bool,
    engine: Option<NativePlaybackEngine>,
    decoders: HashMap<String, Decoder>,
    jitter_buffers: HashMap<String, JitterBuffer>,
    jitter_drainers: HashMap<String, u64>,
    next_jitter_drainer_generation: u64,
    jitter_buffer_us: u32,
    jitter_warning_at_ms: HashMap<String, u64>,
    decoder_warning_at_ms: HashMap<String, u64>,
    user_volumes: HashMap<String, f32>,
    output_gain: f32,
}

impl AudioPlaybackHandle {
    /// Возвращает, отключено ли входящее воспроизведение.
    pub(crate) fn is_muted(&self) -> bool {
        (self.muted)()
    }

    /// Обновляет состояние отключения входящего воспроизведения.
    pub(crate) fn set_muted(&self, muted: bool) {
        let changed_to_muted = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted == muted {
                return;
            }
            inner.muted = muted;
            muted
        };
        let mut muted_signal = self.muted;
        muted_signal.set(muted);

        info!(muted, "native audio playback mute state changed");
        if changed_to_muted {
            self.stop_all();
        } else {
            self.resume();
        }
    }

    /// Устанавливает громкость воспроизведения для каждого пользователя (0-200, где 100 = 100%).
    pub(crate) fn set_user_volume(&self, sender_user_id: &str, volume_percent: u32) {
        let gain = gain_from_percent(volume_percent);
        let mixer = {
            let mut inner = self.inner.borrow_mut();
            inner.user_volumes.insert(sender_user_id.to_owned(), gain);
            inner.engine.as_ref().map(|engine| engine.mixer.clone())
        };
        if let Some(mixer) = mixer {
            update_sender_gain(&mixer, sender_user_id, gain);
        }
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

        let output_gain = gain_from_percent(volume_percent);
        let mixer = {
            let mut inner = self.inner.borrow_mut();
            inner.output_gain = output_gain;
            inner.engine.as_ref().map(|engine| engine.mixer.clone())
        };
        if let Some(mixer) = mixer {
            update_output_gain(&mixer, output_gain);
        }
    }

    /// Возвращает текущую задержку jitter buffer для входящего голоса в микросекундах.
    pub(crate) fn jitter_buffer_us(&self) -> u32 {
        (self.jitter_buffer_us)()
    }

    /// Обновляет задержку jitter buffer для входящего голоса в микросекундах.
    pub(crate) fn set_jitter_buffer_us(&self, buffer_us: u32) {
        let buffer_us = storage::clamp_jitter_buffer_us(buffer_us);
        if *self.jitter_buffer_us.peek() == buffer_us {
            return;
        }

        info!(buffer_us, "inbound voice jitter buffer preference changed");
        storage::save_jitter_buffer_us(buffer_us);
        let mut jitter_buffer_signal = self.jitter_buffer_us;
        jitter_buffer_signal.set(buffer_us);
        self.inner.borrow_mut().jitter_buffer_us = buffer_us;
    }

    /// Сохраняет предпочитаемое устройство вывода и пересоздает native stream.
    pub(crate) fn set_output_device(&self, device: &AudioOutputDevice) {
        if device.device_id.is_empty() {
            self.set_output_device_preference(None, None);
            return;
        }

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
    pub(crate) fn stop_sender(&self, sender_user_id: &str) {
        let mixer = {
            let mut inner = self.inner.borrow_mut();
            inner.decoders.remove(sender_user_id);
            inner.jitter_buffers.remove(sender_user_id);
            inner.jitter_warning_at_ms.remove(sender_user_id);
            inner.decoder_warning_at_ms.remove(sender_user_id);
            inner.engine.as_ref().map(|engine| engine.mixer.clone())
        };
        if let Some(mixer) = mixer {
            remove_sender(&mixer, sender_user_id);
        }
    }

    /// Останавливает все активное воспроизведение.
    pub(crate) fn stop_all(&self) {
        let mixer = {
            let mut inner = self.inner.borrow_mut();
            inner.decoders.clear();
            inner.jitter_buffers.clear();
            inner.jitter_drainers.clear();
            inner.jitter_warning_at_ms.clear();
            inner.decoder_warning_at_ms.clear();
            inner.engine.as_ref().map(|engine| engine.mixer.clone())
        };
        if let Some(mixer) = mixer {
            clear_mixer(&mixer);
        }
        debug!("native audio playback state cleared");
    }

    /// Останавливает голосовое воспроизведение, не прерывая системные notification-звуки.
    pub(crate) fn stop_voice_playback(&self) {
        let mixer = {
            let mut inner = self.inner.borrow_mut();
            inner.decoders.clear();
            inner.jitter_buffers.clear();
            inner.jitter_drainers.clear();
            inner.jitter_warning_at_ms.clear();
            inner.decoder_warning_at_ms.clear();
            inner.engine.as_ref().map(|engine| engine.mixer.clone())
        };
        if let Some(mixer) = mixer {
            clear_voice_senders(&mixer);
        }
        debug!("native voice playback state cleared");
    }

    /// Запускает native output stream, если воспроизведение разрешено.
    pub(crate) fn resume(&self) {
        if self.is_muted() {
            return;
        }
        if let Err(error) = self.ensure_engine() {
            warn!(%error, "failed to resume native audio playback");
        }
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

        self.inner.borrow_mut().engine = None;
        if !self.is_muted()
            && let Err(error) = self.ensure_engine()
        {
            warn!(%error, "failed to switch native audio output device");
        }
    }

    fn ensure_engine(&self) -> Result<(), String> {
        let selected_device_id = self.selected_output_device_id.peek().clone();
        let mut inner = self.inner.borrow_mut();
        if inner
            .engine
            .as_ref()
            .is_some_and(|engine| engine.device_id().as_deref() == selected_device_id.as_deref())
        {
            return Ok(());
        }

        let engine = create_engine(selected_device_id, inner.output_gain, AUDIO_SAMPLE_RATE_HZ)?;
        inner.engine = Some(engine);
        Ok(())
    }
}

/// Предоставляет audio playback state компонентам аутентифицированного приложения.
#[component]
pub(crate) fn AudioPlaybackProvider(children: Element) -> Element {
    let muted = use_signal(|| false);
    let stored_output_device = storage::load_output_device();
    let output_volume_value = storage::load_output_volume_percent();
    let jitter_buffer_us_value = storage::load_jitter_buffer_us();
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
    let jitter_buffer_us = use_signal(move || jitter_buffer_us_value);
    let inner = use_hook(move || {
        Rc::new(RefCell::new(AudioPlaybackInner {
            muted: false,
            engine: None,
            decoders: HashMap::new(),
            jitter_buffers: HashMap::new(),
            jitter_drainers: HashMap::new(),
            next_jitter_drainer_generation: 0,
            jitter_buffer_us: jitter_buffer_us_value,
            jitter_warning_at_ms: HashMap::new(),
            decoder_warning_at_ms: HashMap::new(),
            user_volumes: HashMap::new(),
            output_gain: gain_from_percent(output_volume_value),
        }))
    });
    let handle = AudioPlaybackHandle {
        muted,
        selected_output_device_id,
        selected_output_device_label,
        output_volume_percent,
        jitter_buffer_us,
        inner,
    };
    use_context_provider(move || handle.clone());

    rsx! {
        {children}
    }
}

fn should_emit_sender_warning(
    warnings: &mut HashMap<String, u64>,
    sender_user_id: &str,
    now_ms: u64,
    interval_ms: u64,
) -> bool {
    let last_warning_ms = warnings.get(sender_user_id).copied().unwrap_or_default();
    if last_warning_ms != 0 && now_ms.saturating_sub(last_warning_ms) < interval_ms {
        return false;
    }

    warnings.insert(sender_user_id.to_owned(), now_ms);
    true
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

fn gain_from_percent(volume_percent: u32) -> f32 {
    volume_percent.min(200) as f32 / 100.0
}

fn playback_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn playback_now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
