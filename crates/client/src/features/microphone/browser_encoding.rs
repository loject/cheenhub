//! Browser microphone WebCodecs encoding helpers.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::MessageEvent;
use web_time::{SystemTime, UNIX_EPOCH};

use super::backend::{
    EncodedMicrophoneFrame, MicrophoneCallbacks, MicrophoneCodec, MicrophoneConfig,
    MicrophoneError, MicrophoneFrameCallback, MicrophoneLevel,
};
use super::browser_bindings::{AudioData, AudioEncoder, EncodedAudioChunk};
use super::browser_errors::js_error_message;
use super::vad::{VoiceActivityDetector, rms_level};

const MICROPHONE_ENCODER_QUEUE_WARN_FRAMES: u32 = 8;
const MICROPHONE_WORKLET_DELIVERY_WARN_MS: u64 = 120;
const MICROPHONE_WARNING_INTERVAL_MS: u64 = 5_000;

pub(super) struct BrowserEncoder {
    pub(super) encoder: AudioEncoder,
    pub(super) output_closure: Closure<dyn FnMut(EncodedAudioChunk)>,
    pub(super) error_closure: Closure<dyn FnMut(JsValue)>,
}

pub(super) fn create_encoder(
    encoder_config: &JsValue,
    config: &MicrophoneConfig,
    on_frame: MicrophoneFrameCallback,
) -> Result<BrowserEncoder, MicrophoneError> {
    let sequence = Rc::new(Cell::new(0_u64));
    let output_sequence = sequence.clone();
    let sample_rate_hz = config.sample_rate_hz;
    let channels = config.channels;
    let output_closure = Closure::wrap(Box::new(move |chunk: EncodedAudioChunk| {
        let byte_length = chunk.byte_length();
        let destination = Uint8Array::new_with_length(byte_length);
        if chunk.copy_to(&destination).is_err() {
            return;
        }
        let mut bytes = vec![0; byte_length as usize];
        destination.copy_to(&mut bytes);
        let sequence = output_sequence.get();
        output_sequence.set(sequence.saturating_add(1));
        on_frame(EncodedMicrophoneFrame {
            sequence,
            timestamp_us: chunk.timestamp().max(0.0) as u64,
            duration_us: chunk.duration().unwrap_or(0.0).max(0.0) as u32,
            codec: MicrophoneCodec::Opus,
            sample_rate_hz,
            channels,
            bytes,
        });
    }) as Box<dyn FnMut(EncodedAudioChunk)>);
    let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
        warn!(
            error = %js_error_message(error),
            "browser microphone encoder failed"
        );
    }) as Box<dyn FnMut(JsValue)>);
    let init = Object::new();
    set_property(&init, "error", error_closure.as_ref())?;
    set_property(&init, "output", output_closure.as_ref())?;
    let encoder = AudioEncoder::new(&init.into()).map_err(microphone_error)?;
    encoder
        .configure(encoder_config)
        .map_err(microphone_error)?;

    Ok(BrowserEncoder {
        encoder,
        output_closure,
        error_closure,
    })
}

pub(super) fn microphone_message_closure(
    encoder: AudioEncoder,
    callbacks: MicrophoneCallbacks,
    config: MicrophoneConfig,
    sample_rate_hz: u32,
    closed: Rc<Cell<bool>>,
) -> Closure<dyn FnMut(MessageEvent)> {
    let detector = Rc::new(RefCell::new(VoiceActivityDetector::new(config.clone())));
    let diagnostics = Rc::new(MicrophoneWorkletDiagnostics {
        encoder_queue_warning_ms: Rc::new(Cell::new(0)),
        chunk_timing: Rc::new(RefCell::new(MicrophoneChunkTiming::default())),
    });
    Closure::wrap(Box::new(move |event: MessageEvent| {
        if closed.get() {
            return;
        }
        if let Err(error) = handle_worklet_message(
            event.data(),
            &encoder,
            &detector,
            &callbacks,
            sample_rate_hz,
            config.input_gain,
            diagnostics.as_ref(),
        ) {
            warn!(
                %error,
                "failed to handle microphone audio worklet chunk"
            );
        }
    }) as Box<dyn FnMut(MessageEvent)>)
}

pub(super) fn encoder_config(sample_rate_hz: u32, channels: u8, bitrate_bps: u32) -> JsValue {
    let object = Object::new();
    let _ = Reflect::set(
        &object,
        &JsValue::from_str("codec"),
        &JsValue::from_str("opus"),
    );
    let _ = Reflect::set(
        &object,
        &JsValue::from_str("sampleRate"),
        &JsValue::from_f64(f64::from(sample_rate_hz)),
    );
    let _ = Reflect::set(
        &object,
        &JsValue::from_str("numberOfChannels"),
        &JsValue::from_f64(f64::from(channels)),
    );
    let _ = Reflect::set(
        &object,
        &JsValue::from_str("bitrate"),
        &JsValue::from_f64(f64::from(bitrate_bps)),
    );
    object.into()
}

pub(super) fn close_encoder_lossy(encoder: &AudioEncoder) {
    if let Err(error) = encoder.close() {
        warn!(
            error = %js_error_message(error),
            "failed to close microphone encoder"
        );
    }
}

fn handle_worklet_message(
    data: JsValue,
    encoder: &AudioEncoder,
    detector: &Rc<RefCell<VoiceActivityDetector>>,
    callbacks: &MicrophoneCallbacks,
    sample_rate_hz: u32,
    input_gain: f32,
    diagnostics: &MicrophoneWorkletDiagnostics,
) -> Result<(), MicrophoneError> {
    let chunk = PcmChunk::from_message(data)?;
    if chunk.samples.is_empty() {
        return Ok(());
    }

    let duration_us = duration_us(chunk.samples.len(), sample_rate_hz);
    warn_if_worklet_delivery_is_late(&chunk, duration_us, &diagnostics.chunk_timing);
    let mut samples = chunk.samples;
    apply_input_gain(&mut samples, input_gain);
    let rms = rms_level(&samples);
    let previous_active = detector.borrow().is_active();
    let active = detector.borrow_mut().update(rms, duration_us);
    if active != previous_active {
        debug!(
            rms,
            active,
            threshold = detector_threshold(detector),
            timestamp_us = chunk.timestamp_us,
            "microphone voice activation changed"
        );
    }
    (callbacks.on_level)(MicrophoneLevel {
        rms,
        active,
        threshold: detector_threshold(detector),
        timestamp_us: chunk.timestamp_us,
    });

    if !active {
        return Ok(());
    }

    let audio = audio_data_from_samples(&samples, sample_rate_hz, chunk.timestamp_us)?;
    let encode_result = encoder.encode(audio.as_ref());
    let close_result = audio.close();
    let queue_size = encoder.encode_queue_size();
    if queue_size >= MICROPHONE_ENCODER_QUEUE_WARN_FRAMES
        && should_emit_cell_warning(
            &diagnostics.encoder_queue_warning_ms,
            browser_now_ms(),
            MICROPHONE_WARNING_INTERVAL_MS,
        )
    {
        warn!(
            queue_size,
            duration_us,
            timestamp_us = chunk.timestamp_us,
            "microphone encoder queue is backing up"
        );
    }
    encode_result.map_err(microphone_error)?;
    close_result.map_err(microphone_error)?;
    Ok(())
}

struct MicrophoneWorkletDiagnostics {
    encoder_queue_warning_ms: Rc<Cell<u64>>,
    chunk_timing: Rc<RefCell<MicrophoneChunkTiming>>,
}

#[derive(Default)]
struct MicrophoneChunkTiming {
    first_timestamp_us: Option<u64>,
    first_wall_ms: u64,
    last_timestamp_us: Option<u64>,
    last_warning_ms: u64,
}

struct PcmChunk {
    samples: Vec<f32>,
    timestamp_us: u64,
}

fn warn_if_worklet_delivery_is_late(
    chunk: &PcmChunk,
    duration_us: u32,
    timing: &Rc<RefCell<MicrophoneChunkTiming>>,
) {
    let now_ms = browser_now_ms();
    let mut timing = timing.borrow_mut();
    let first_timestamp_us = match timing.first_timestamp_us {
        Some(timestamp_us) => timestamp_us,
        None => {
            timing.first_timestamp_us = Some(chunk.timestamp_us);
            timing.first_wall_ms = now_ms;
            timing.last_timestamp_us = Some(chunk.timestamp_us);
            return;
        }
    };

    let expected_wall_ms = timing.first_wall_ms.saturating_add(
        chunk
            .timestamp_us
            .saturating_sub(first_timestamp_us)
            .checked_div(1_000)
            .unwrap_or_default(),
    );
    let delivery_late_ms = now_ms.saturating_sub(expected_wall_ms);
    let previous_timestamp_us = timing.last_timestamp_us.replace(chunk.timestamp_us);
    let timestamp_gap_us = previous_timestamp_us
        .map(|previous_timestamp_us| chunk.timestamp_us.saturating_sub(previous_timestamp_us))
        .unwrap_or_default();
    let timestamp_gap_ms = timestamp_gap_us.checked_div(1_000).unwrap_or_default();
    let expected_gap_ms = u64::from(duration_us)
        .checked_div(1_000)
        .unwrap_or_default();
    if delivery_late_ms < MICROPHONE_WORKLET_DELIVERY_WARN_MS {
        return;
    }
    if timing.last_warning_ms != 0
        && now_ms.saturating_sub(timing.last_warning_ms) < MICROPHONE_WARNING_INTERVAL_MS
    {
        return;
    }

    timing.last_warning_ms = now_ms;
    warn!(
        delivery_late_ms,
        timestamp_gap_ms,
        expected_gap_ms,
        timestamp_us = chunk.timestamp_us,
        "microphone audio worklet chunk reached main thread late"
    );
}

impl PcmChunk {
    fn from_message(data: JsValue) -> Result<Self, MicrophoneError> {
        let samples = Reflect::get(&data, &JsValue::from_str("samples"))
            .map_err(microphone_error)?
            .dyn_into::<Float32Array>()
            .map_err(|_| MicrophoneError::new("AudioWorklet вернул некорректные PCM samples."))?
            .to_vec();
        let timestamp_us = Reflect::get(&data, &JsValue::from_str("timestampUs"))
            .map_err(microphone_error)?
            .as_f64()
            .unwrap_or(0.0)
            .max(0.0) as u64;

        Ok(Self {
            samples,
            timestamp_us,
        })
    }
}

fn audio_data_from_samples(
    samples: &[f32],
    sample_rate_hz: u32,
    timestamp_us: u64,
) -> Result<AudioData, MicrophoneError> {
    let data = Float32Array::from(samples);
    let init = Object::new();
    set_property(&init, "format", &JsValue::from_str("f32-planar"))?;
    set_property(
        &init,
        "sampleRate",
        &JsValue::from_f64(f64::from(sample_rate_hz)),
    )?;
    set_property(
        &init,
        "numberOfFrames",
        &JsValue::from_f64(samples.len() as f64),
    )?;
    set_property(&init, "numberOfChannels", &JsValue::from_f64(1.0))?;
    set_property(&init, "timestamp", &JsValue::from_f64(timestamp_us as f64))?;
    set_property(&init, "data", data.as_ref())?;
    AudioData::new(&init.into()).map_err(microphone_error)
}

fn apply_input_gain(samples: &mut [f32], input_gain: f32) {
    if (input_gain - 1.0).abs() < f32::EPSILON {
        return;
    }

    for sample in samples {
        *sample = (*sample * input_gain).clamp(-1.0, 1.0);
    }
}

fn detector_threshold(detector: &Rc<RefCell<VoiceActivityDetector>>) -> f32 {
    detector.borrow().config().vad_threshold
}

fn duration_us(frames: usize, sample_rate_hz: u32) -> u32 {
    ((frames as u64)
        .saturating_mul(1_000_000)
        .checked_div(u64::from(sample_rate_hz.max(1)))
        .unwrap_or(0))
    .min(u64::from(u32::MAX)) as u32
}

fn should_emit_cell_warning(last_warning_ms: &Cell<u64>, now_ms: u64, interval_ms: u64) -> bool {
    let last_ms = last_warning_ms.get();
    if last_ms != 0 && now_ms.saturating_sub(last_ms) < interval_ms {
        return false;
    }

    last_warning_ms.set(now_ms);
    true
}

fn browser_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn microphone_error(error: JsValue) -> MicrophoneError {
    MicrophoneError::new(js_error_message(error))
}

fn set_property(object: &Object, name: &str, value: &JsValue) -> Result<(), MicrophoneError> {
    Reflect::set(object, &JsValue::from_str(name), value)
        .map(|_| ())
        .map_err(microphone_error)
}
