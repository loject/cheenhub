//! Audio playback context provider.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{AudioBufferSourceNode, AudioContext, GainNode};

use super::browser_bindings::{AudioData, AudioDecoder, EncodedAudioChunk};
use super::browser_helpers::{
    apply_output_device_to_context, js_error_message, set_property, stop_audio_source,
};
use super::output_devices::AudioOutputDevice;
use super::storage;

const INITIAL_PLAYBACK_BUFFER_SECONDS: f64 = 0.12;
const CONTINUOUS_PLAYBACK_MARGIN_SECONDS: f64 = 0.02;
/// Encoded playback codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaybackCodec {
    /// Opus audio.
    Opus,
}

/// Encoded voice frame prepared for playback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceFrame {
    /// Authenticated sender identifier.
    pub(crate) sender_user_id: String,
    /// Sender-local packet sequence.
    #[allow(dead_code)]
    pub(crate) sequence: u64,
    /// Capture or encode timestamp in microseconds.
    pub(crate) timestamp_us: u64,
    /// Frame duration in microseconds.
    pub(crate) duration_us: u32,
    /// Encoded codec.
    pub(crate) codec: PlaybackCodec,
    /// Encoded frame bytes.
    pub(crate) bytes: Vec<u8>,
}

/// Context handle for browser audio playback.
#[derive(Clone)]
pub(crate) struct AudioPlaybackHandle {
    muted: Signal<bool>,
    selected_output_device_id: Signal<Option<String>>,
    selected_output_device_label: Signal<Option<String>>,
    inner: Rc<RefCell<AudioPlaybackInner>>,
}

struct AudioPlaybackInner {
    context: Option<AudioContext>,
    muted: bool,
    senders: HashMap<String, SenderPlayback>,
    scheduled_sources: HashMap<String, Vec<ScheduledAudioSource>>,
    scheduled_until: HashMap<String, f64>,
    /// Per-user gain values (0.0–2.0, default 1.0). Persisted so volumes set
    /// before the first frame are applied when the sender is first created.
    user_volumes: HashMap<String, f64>,
}

struct ScheduledAudioSource {
    source: AudioBufferSourceNode,
    end_time: f64,
}

struct SenderPlayback {
    decoder: AudioDecoder,
    gain_node: GainNode,
    _output_closure: Closure<dyn FnMut(AudioData)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
}

impl AudioPlaybackHandle {
    /// Plays one encoded voice frame.
    pub(crate) fn play_voice_frame(&self, frame: VoiceFrame) {
        if frame.codec != PlaybackCodec::Opus || frame.bytes.is_empty() {
            return;
        }
        if self.is_muted() {
            return;
        }
        if let Err(error) = self.play(frame) {
            warn!(
                error = %js_error_message(error),
                "failed to play inbound voice frame"
            );
        }
    }

    /// Returns whether inbound playback is muted.
    pub(crate) fn is_muted(&self) -> bool {
        (self.muted)()
    }

    /// Updates inbound playback mute state.
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

    /// Sets per-user playback volume (0–200, where 100 = 100%).
    pub(crate) fn set_user_volume(&self, sender_user_id: &str, volume_percent: u32) {
        let gain = f64::from(volume_percent) / 100.0;
        let mut inner = self.inner.borrow_mut();
        inner.user_volumes.insert(sender_user_id.to_owned(), gain);
        if let Some(sender) = inner.senders.get(sender_user_id) {
            sender.gain_node.gain().set_value(gain as f32);
        }
    }

    /// Stores the preferred audio output device and applies it to the active context.
    pub(crate) fn set_output_device(&self, device: &AudioOutputDevice) {
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
        let sender_ids = {
            let inner = self.inner.borrow();
            let mut sender_ids = inner.senders.keys().cloned().collect::<Vec<_>>();
            for sender_id in inner.scheduled_sources.keys() {
                if !sender_ids.iter().any(|known_id| known_id == sender_id) {
                    sender_ids.push(sender_id.clone());
                }
            }
            sender_ids
        };
        for sender_id in sender_ids {
            self.stop_sender(&sender_id);
        }
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

    fn play(&self, frame: VoiceFrame) -> Result<(), JsValue> {
        let context = self.context()?;
        let mut inner = self.inner.borrow_mut();
        if inner.muted {
            return Ok(());
        }
        if !inner.senders.contains_key(&frame.sender_user_id) {
            let initial_gain = inner
                .user_volumes
                .get(&frame.sender_user_id)
                .copied()
                .unwrap_or(1.0);
            let sender = create_sender_playback(
                frame.sender_user_id.clone(),
                context.clone(),
                self.inner.clone(),
                initial_gain,
            )?;
            inner.senders.insert(frame.sender_user_id.clone(), sender);
        }
        let Some(sender) = inner.senders.get(&frame.sender_user_id) else {
            return Ok(());
        };
        let chunk = encoded_audio_chunk(&frame)?;
        sender.decoder.decode(&chunk)
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
        if let Some(device_id) = device_id {
            apply_output_device_to_context(context, device_id);
        }
    }
}

/// Provides audio playback to authenticated app features.
#[component]
pub(crate) fn AudioPlaybackProvider(children: Element) -> Element {
    let muted = use_signal(|| false);
    let stored_output_device = storage::load_output_device();
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
    let handle = AudioPlaybackHandle {
        muted,
        selected_output_device_id,
        selected_output_device_label,
        inner: Rc::new(RefCell::new(AudioPlaybackInner {
            context: None,
            muted: false,
            senders: HashMap::new(),
            scheduled_sources: HashMap::new(),
            scheduled_until: HashMap::new(),
            user_volumes: HashMap::new(),
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

fn create_sender_playback(
    sender_user_id: String,
    context: AudioContext,
    inner: Rc<RefCell<AudioPlaybackInner>>,
    initial_gain: f64,
) -> Result<SenderPlayback, JsValue> {
    let gain_node = context.create_gain()?;
    gain_node.gain().set_value(initial_gain as f32);
    gain_node.connect_with_audio_node(&context.destination())?;

    let output_sender_id = sender_user_id.clone();
    let output_closure = Closure::wrap(Box::new(move |audio: AudioData| {
        if let Err(error) = schedule_audio_data(&context, &inner, &output_sender_id, &audio) {
            warn!(
                error = %js_error_message(error),
                sender_user_id = %output_sender_id,
                "failed to schedule decoded audio"
            );
        }
        let _ = audio.close();
    }) as Box<dyn FnMut(AudioData)>);
    let error_sender_id = sender_user_id.clone();
    let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
        warn!(
            error = %js_error_message(error),
            sender_user_id = %error_sender_id,
            "audio decoder failed"
        );
    }) as Box<dyn FnMut(JsValue)>);
    let init = Object::new();
    Reflect::set(&init, &JsValue::from_str("output"), output_closure.as_ref())?;
    Reflect::set(&init, &JsValue::from_str("error"), error_closure.as_ref())?;
    let decoder = AudioDecoder::new(&init.into())?;
    decoder.configure(&decoder_config())?;

    Ok(SenderPlayback {
        decoder,
        gain_node,
        _output_closure: output_closure,
        _error_closure: error_closure,
    })
}

fn decoder_config() -> JsValue {
    let object = Object::new();
    set_property(&object, "codec", &JsValue::from_str("opus"));
    set_property(&object, "sampleRate", &JsValue::from_f64(48_000.0));
    set_property(&object, "numberOfChannels", &JsValue::from_f64(1.0));
    object.into()
}

fn encoded_audio_chunk(frame: &VoiceFrame) -> Result<EncodedAudioChunk, JsValue> {
    let data = Uint8Array::from(frame.bytes.as_slice());
    let init = Object::new();
    Reflect::set(&init, &JsValue::from_str("type"), &JsValue::from_str("key"))?;
    Reflect::set(
        &init,
        &JsValue::from_str("timestamp"),
        &JsValue::from_f64(frame.timestamp_us as f64),
    )?;
    Reflect::set(
        &init,
        &JsValue::from_str("duration"),
        &JsValue::from_f64(f64::from(frame.duration_us)),
    )?;
    Reflect::set(&init, &JsValue::from_str("data"), data.as_ref())?;
    EncodedAudioChunk::new(&init.into())
}

fn schedule_audio_data(
    context: &AudioContext,
    inner: &Rc<RefCell<AudioPlaybackInner>>,
    sender_user_id: &str,
    audio: &AudioData,
) -> Result<(), JsValue> {
    if inner.borrow().muted {
        return Ok(());
    }

    let frames = audio.number_of_frames();
    if frames == 0 {
        return Ok(());
    }
    let channels = audio.number_of_channels().max(1);
    let sample_rate = audio.sample_rate().max(1.0) as f32;
    let buffer = context.create_buffer(channels, frames, sample_rate)?;

    for channel in 0..channels {
        let samples = Float32Array::new_with_length(frames);
        audio.copy_to(&samples, &copy_options(channel))?;
        buffer.copy_to_channel_with_f32_array(&samples, channel as i32)?;
    }

    let source = context.create_buffer_source()?;
    source.set_buffer(Some(&buffer));
    let gain_node = inner
        .borrow()
        .senders
        .get(sender_user_id)
        .map(|s| s.gain_node.clone());
    match gain_node {
        Some(gain) => source.connect_with_audio_node(&gain)?,
        None => source.connect_with_audio_node(&context.destination())?,
    };

    let now = context.current_time();
    let mut inner = inner.borrow_mut();
    let previous_until = inner.scheduled_until.get(sender_user_id).copied();
    let start_at = match previous_until {
        Some(previous_until) if previous_until > now => {
            previous_until.max(now + CONTINUOUS_PLAYBACK_MARGIN_SECONDS)
        }
        _ => {
            debug!(
                %sender_user_id,
                buffer_ms = INITIAL_PLAYBACK_BUFFER_SECONDS * 1000.0,
                "priming inbound voice playback buffer"
            );
            now + INITIAL_PLAYBACK_BUFFER_SECONDS
        }
    };
    let duration = f64::from(frames) / f64::from(sample_rate);
    let end_time = start_at + duration;
    inner
        .scheduled_until
        .insert(sender_user_id.to_owned(), end_time);
    source.start_with_when(start_at)?;
    let sources = inner
        .scheduled_sources
        .entry(sender_user_id.to_owned())
        .or_default();
    sources.retain(|source| source.end_time > now);
    sources.push(ScheduledAudioSource { source, end_time });

    Ok(())
}

fn copy_options(plane_index: u32) -> JsValue {
    let options = Object::new();
    set_property(&options, "format", &JsValue::from_str("f32-planar"));
    set_property(
        &options,
        "planeIndex",
        &JsValue::from_f64(f64::from(plane_index)),
    );
    options.into()
}
