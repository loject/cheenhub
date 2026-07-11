//! Web-провайдер контекста воспроизведения аудио.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]
#[path = "browser_bindings.rs"]
mod browser_bindings;
#[path = "browser_diagnostics.rs"]
mod browser_diagnostics;
#[path = "browser_helpers.rs"]
mod browser_helpers;
#[path = "jitter_buffer.rs"]
mod jitter_buffer;
#[path = "jitter_runtime.rs"]
mod jitter_runtime;
#[path = "playback_pipeline.rs"]
mod playback_pipeline;
#[path = "web_notifications.rs"]
mod web_notifications;
#[path = "web_voice_playback.rs"]
mod web_voice_playback;
use self::browser_diagnostics::{
    AudioPlaybackDiagnostics, DecodeInputTiming, diagnostics_enabled, elapsed_us_since,
};
use self::browser_helpers::{apply_output_device_to_context, js_error_message, stop_audio_source};
use self::jitter_buffer::JitterBuffer;
use self::playback_pipeline::{
    ScheduledAudioSource, SenderPlayback, create_sender_playback, encoded_audio_chunk,
};
use self::web_notifications::ConnectionSignalLoopState;
use super::backend::VoiceFrame;
use super::output_devices::AudioOutputDevice;
use super::storage;
use dioxus::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::AudioContext;
use web_time::{Instant, SystemTime, UNIX_EPOCH};

const AUDIO_DECODER_QUEUE_WARN_FRAMES: u32 = 8;
pub(super) const AUDIO_PLAYBACK_WARNING_INTERVAL_MS: u64 = 5_000;

/// Контекстный хэндл для браузерного аудиовоспроизведения.
#[derive(Clone)]
pub(crate) struct AudioPlaybackHandle {
    muted: Signal<bool>,
    selected_output_device_id: Signal<Option<String>>,
    selected_output_device_label: Signal<Option<String>>,
    output_volume_percent: Signal<u32>,
    jitter_buffer_ms: Signal<u32>,
    pub(super) inner: Rc<RefCell<AudioPlaybackInner>>,
}

pub(super) struct AudioPlaybackInner {
    pub(in crate::features::audio_playback::web) context: Option<AudioContext>,
    pub(in crate::features::audio_playback::web) muted: bool,
    pub(in crate::features::audio_playback::web) senders: HashMap<String, SenderPlayback>,
    pub(in crate::features::audio_playback::web) jitter_buffers: HashMap<String, JitterBuffer>,
    pub(in crate::features::audio_playback::web) jitter_drainers: HashMap<String, u64>,
    pub(in crate::features::audio_playback::web) next_jitter_drainer_generation: u64,
    pub(in crate::features::audio_playback::web) jitter_buffer_ms: u32,
    pub(in crate::features::audio_playback::web) jitter_warning_at_ms: HashMap<String, u64>,
    pub(in crate::features::audio_playback::web) decoder_queue_warning_at_ms: HashMap<String, u64>,
    pub(in crate::features::audio_playback::web) playback_schedule_warning_at: HashMap<String, f64>,
    pub(in crate::features::audio_playback::web) diagnostics: AudioPlaybackDiagnostics,
    pub(in crate::features::audio_playback::web) scheduled_sources:
        HashMap<String, Vec<ScheduledAudioSource>>,
    pub(in crate::features::audio_playback::web) scheduled_until: HashMap<String, f64>,
    pub(in crate::features::audio_playback::web) connection_signal_loop: ConnectionSignalLoopState,
    /// Значения усиления для каждого пользователя (0.0–2.0, по умолчанию 1.0).
    /// Сохраняется, чтобы громкость, заданная до первого фрейма, применялась при первом создании отправителя.
    pub(in crate::features::audio_playback::web) user_volumes: HashMap<String, f64>,
    pub(in crate::features::audio_playback::web) output_gain: f64,
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

        if changed_to_muted {
            self.stop_all();
        } else {
            self.resume();
        }
    }

    /// Устанавливает громкость воспроизведения для каждого пользователя (0–200, где 100 = 100%).
    pub(crate) fn set_user_volume(&self, sender_user_id: &str, volume_percent: u32) {
        let gain = gain_from_percent(volume_percent);
        let mut inner = self.inner.borrow_mut();
        inner.user_volumes.insert(sender_user_id.to_owned(), gain);
        let effective_gain = gain * inner.output_gain;
        if let Some(sender) = inner.senders.get(sender_user_id) {
            sender.gain_node.gain().set_value(effective_gain as f32);
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
        let next_gain = gain_from_percent(volume_percent);
        let mut inner = self.inner.borrow_mut();
        inner.output_gain = next_gain;
        for (sender_user_id, sender) in &inner.senders {
            let sender_gain = inner
                .user_volumes
                .get(sender_user_id)
                .copied()
                .unwrap_or(1.0);
            sender
                .gain_node
                .gain()
                .set_value((sender_gain * next_gain) as f32);
        }
    }

    /// Returns the current inbound voice jitter buffer delay in milliseconds.
    pub(crate) fn jitter_buffer_ms(&self) -> u32 {
        (self.jitter_buffer_ms)()
    }

    /// Updates the inbound voice jitter buffer delay in milliseconds.
    pub(crate) fn set_jitter_buffer_ms(&self, buffer_ms: u32) {
        let buffer_ms = storage::clamp_jitter_buffer_ms(buffer_ms);
        if *self.jitter_buffer_ms.peek() == buffer_ms {
            return;
        }

        info!(buffer_ms, "inbound voice jitter buffer preference changed");
        storage::save_jitter_buffer_ms(buffer_ms);
        let mut jitter_buffer_signal = self.jitter_buffer_ms;
        jitter_buffer_signal.set(buffer_ms);
        self.inner.borrow_mut().jitter_buffer_ms = buffer_ms;
    }

    /// Stores the preferred audio output device and applies it to the active context.
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

    /// Reconciles a stored output device preference against enumerated devices.
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

    /// Returns the currently preferred audio output device ID.
    pub(crate) fn output_device_id(&self) -> Option<String> {
        (self.selected_output_device_id)()
    }

    /// Stops playback state for one sender.
    pub(crate) fn stop_sender(&self, sender_user_id: &str) {
        let (sender, sources) = {
            let mut inner = self.inner.borrow_mut();
            let sender = inner.senders.remove(sender_user_id);
            inner.jitter_buffers.remove(sender_user_id);
            inner.jitter_warning_at_ms.remove(sender_user_id);
            inner.decoder_queue_warning_at_ms.remove(sender_user_id);
            inner.playback_schedule_warning_at.remove(sender_user_id);
            inner.diagnostics.remove_sender(sender_user_id);
            let sources = inner
                .scheduled_sources
                .remove(sender_user_id)
                .unwrap_or_default();
            inner.scheduled_until.remove(sender_user_id);
            (sender, sources)
        };

        for scheduled in sources {
            if let Err(error) = stop_audio_source(&scheduled.source) {
                warn!(
                    error = %js_error_message(error),
                    %sender_user_id,
                    "failed to stop scheduled audio source"
                );
            }
        }

        if let Some(sender) = sender
            && let Err(error) = sender.decoder.close()
        {
            warn!(
                error = %js_error_message(error),
                %sender_user_id,
                "failed to close audio decoder"
            );
        }
    }

    /// Stops all active playback state.
    pub(crate) fn stop_all(&self) {
        self.stop_connection_signal_loop();
        self.stop_voice_playback();
    }

    /// Resumes the browser audio context after a user gesture.
    pub(crate) fn resume(&self) {
        if self.is_muted() {
            return;
        }
        let Ok(context) = self.context() else {
            return;
        };
        if let Ok(promise) = context.resume() {
            spawn_local(async move {
                if let Err(error) = JsFuture::from(promise).await {
                    warn!(
                        error = %js_error_message(error),
                        "failed to resume audio playback context"
                    );
                }
            });
        }
    }

    pub(super) fn decode_voice_frame(&self, frame: VoiceFrame) -> Result<(), JsValue> {
        let should_record_diagnostics = diagnostics_enabled();
        let started_at = should_record_diagnostics.then(Instant::now);
        let sender_user_id = frame.sender_user_id.clone();
        let sequence = frame.sequence;
        let timestamp_us = frame.timestamp_us;
        let payload_bytes = frame.bytes.len();
        let context_started_at = should_record_diagnostics.then(Instant::now);
        let context = self.context()?;
        let context_elapsed_us = elapsed_us_since(&context_started_at);
        let chunk_started_at = should_record_diagnostics.then(Instant::now);
        let chunk = encoded_audio_chunk(&frame)?;
        let chunk_elapsed_us = elapsed_us_since(&chunk_started_at);
        let sender_started_at = should_record_diagnostics.then(Instant::now);
        let decoder = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted {
                return Ok(());
            }
            if !inner.senders.contains_key(&sender_user_id) {
                let initial_gain = inner
                    .user_volumes
                    .get(&sender_user_id)
                    .copied()
                    .unwrap_or(1.0)
                    * inner.output_gain;
                let sender = create_sender_playback(
                    sender_user_id.clone(),
                    context.clone(),
                    self.inner.clone(),
                    initial_gain,
                )?;
                inner.senders.insert(sender_user_id.clone(), sender);
            }
            inner
                .senders
                .get(&sender_user_id)
                .map(|sender| sender.decoder.clone())
        };
        let sender_elapsed_us = elapsed_us_since(&sender_started_at);
        let Some(decoder) = decoder else {
            return Ok(());
        };
        let decode_started_at = should_record_diagnostics.then(Instant::now);
        let result = decoder.decode(&chunk);
        let decode_elapsed_us = elapsed_us_since(&decode_started_at);
        let queue_size = decoder.decode_queue_size();
        if should_record_diagnostics {
            self.inner.borrow_mut().diagnostics.record_decode_input(
                &sender_user_id,
                DecodeInputTiming {
                    timestamp_us,
                    payload_bytes,
                    total_elapsed_us: elapsed_us_since(&started_at),
                    context_elapsed_us,
                    chunk_elapsed_us,
                    sender_elapsed_us,
                    decode_elapsed_us,
                    queue_size,
                },
            );
        }
        if queue_size >= AUDIO_DECODER_QUEUE_WARN_FRAMES {
            let should_warn = {
                let mut inner = self.inner.borrow_mut();
                should_emit_sender_warning(
                    &mut inner.decoder_queue_warning_at_ms,
                    &sender_user_id,
                    audio_playback_now_ms(),
                    AUDIO_PLAYBACK_WARNING_INTERVAL_MS,
                )
            };
            if should_warn {
                warn!(
                    %sender_user_id,
                    sequence,
                    queue_size,
                    "inbound audio decoder queue is backing up"
                );
            }
        }
        result
    }

    fn context(&self) -> Result<AudioContext, JsValue> {
        if let Some(context) = self.inner.borrow().context.clone() {
            return Ok(context);
        }

        let context = AudioContext::new()?;
        if let Some(device_id) = self.selected_output_device_id.peek().clone() {
            apply_output_device_to_context(context.clone(), device_id);
        }
        self.inner.borrow_mut().context = Some(context.clone());
        Ok(context)
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
        id_signal.set(device_id.clone());
        label_signal.set(label);

        let Some(context) = self.inner.borrow().context.clone() else {
            return;
        };
        apply_output_device_to_context(context, device_id.unwrap_or_default());
    }
}

/// Provides audio playback to authenticated app features.
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
    let output_gain = gain_from_percent(output_volume_value);
    let handle = AudioPlaybackHandle {
        muted,
        selected_output_device_id,
        selected_output_device_label,
        output_volume_percent,
        jitter_buffer_ms,
        inner: Rc::new(RefCell::new(AudioPlaybackInner {
            context: None,
            muted: false,
            senders: HashMap::new(),
            jitter_buffers: HashMap::new(),
            jitter_drainers: HashMap::new(),
            next_jitter_drainer_generation: 0,
            jitter_buffer_ms: jitter_buffer_ms_value,
            jitter_warning_at_ms: HashMap::new(),
            decoder_queue_warning_at_ms: HashMap::new(),
            playback_schedule_warning_at: HashMap::new(),
            diagnostics: AudioPlaybackDiagnostics::new(),
            scheduled_sources: HashMap::new(),
            scheduled_until: HashMap::new(),
            connection_signal_loop: ConnectionSignalLoopState::default(),
            user_volumes: HashMap::new(),
            output_gain,
        })),
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

pub(super) fn should_emit_sender_warning(
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

pub(super) fn audio_playback_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn gain_from_percent(volume_percent: u32) -> f64 {
    f64::from(volume_percent.min(200)) / 100.0
}
