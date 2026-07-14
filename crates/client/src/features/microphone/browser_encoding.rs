//! Browser microphone WebCodecs encoding helpers.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::MessageEvent;
use web_time::Instant;

use super::backend::{
    EncodedMicrophoneFrame, MicrophoneCallbacks, MicrophoneCodec, MicrophoneConfig,
    MicrophoneError, MicrophoneFrameCallback, MicrophoneLevel,
};
use super::browser_bindings::{AudioData, AudioEncoder, EncodedAudioChunk};
use super::browser_diagnostics::{
    EncoderInputTiming, EncoderOutputTiming, MicrophoneEncoderOutputDiagnostics,
    MicrophoneWorkletDiagnostics, WorkletDeliveryTiming, WorkletProcessingTiming,
    WorkletProcessorProfileTiming, enabled as microphone_diagnostics_enabled,
};
use super::browser_errors::js_error_message;
use super::core::{apply_input_gain, duration_us};
use super::vad::{VoiceActivityDetector, rms_level};

const MICROPHONE_ENCODER_QUEUE_WARN_FRAMES: u32 = 8;

pub(super) struct BrowserEncoder {
    pub(super) encoder: AudioEncoder,
    pub(super) diagnostics: Rc<MicrophoneEncoderOutputDiagnostics>,
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
    let output_diagnostics = Rc::new(MicrophoneEncoderOutputDiagnostics::new());
    let output_callback_diagnostics = output_diagnostics.clone();
    let output_closure = Closure::wrap(Box::new(move |chunk: EncodedAudioChunk| {
        let diagnostics_enabled = microphone_diagnostics_enabled();
        let started_at = diagnostics_enabled.then(Instant::now);
        let timestamp_us = chunk.timestamp().max(0.0) as u64;
        let duration_us = chunk.duration().unwrap_or(0.0).max(0.0) as u32;
        let byte_length = chunk.byte_length();
        let destination = Uint8Array::new_with_length(byte_length);
        let copy_chunk_started_at = diagnostics_enabled.then(Instant::now);
        if chunk.copy_to(&destination).is_err() {
            return;
        }
        let copy_chunk_elapsed_us = elapsed_us_since(&copy_chunk_started_at);
        let vec_copy_started_at = diagnostics_enabled.then(Instant::now);
        let mut bytes = vec![0; byte_length as usize];
        destination.copy_to(&mut bytes);
        let vec_copy_elapsed_us = elapsed_us_since(&vec_copy_started_at);
        let sequence = output_sequence.get();
        output_sequence.set(sequence.saturating_add(1));
        let on_frame_started_at = diagnostics_enabled.then(Instant::now);
        on_frame(EncodedMicrophoneFrame {
            sequence,
            timestamp_us,
            duration_us,
            codec: MicrophoneCodec::Opus,
            sample_rate_hz,
            channels,
            bytes,
        });
        let on_frame_elapsed_us = elapsed_us_since(&on_frame_started_at);
        if diagnostics_enabled {
            output_callback_diagnostics.record_output(EncoderOutputTiming {
                duration_us,
                payload_bytes: byte_length,
                total_elapsed_us: elapsed_us_since(&started_at),
                copy_chunk_elapsed_us,
                vec_copy_elapsed_us,
                on_frame_elapsed_us,
            });
        }
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
        diagnostics: output_diagnostics,
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
    encoder_diagnostics: Rc<MicrophoneEncoderOutputDiagnostics>,
) -> Closure<dyn FnMut(MessageEvent)> {
    let detector = Rc::new(RefCell::new(VoiceActivityDetector::new(config.clone())));
    let diagnostics = Rc::new(MicrophoneWorkletDiagnostics::new());
    Closure::wrap(Box::new(move |event: MessageEvent| {
        if closed.get() {
            return;
        }
        if let Err(error) = handle_worklet_message(
            event.data(),
            WorkletMessageContext {
                encoder: &encoder,
                detector: &detector,
                callbacks: &callbacks,
                sample_rate_hz,
                input_gain: config.input_gain,
                diagnostics: diagnostics.as_ref(),
                encoder_diagnostics: encoder_diagnostics.as_ref(),
            },
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

    // Opus-специфичные параметры для устойчивости голоса к плохой сети.
    //
    // - useinbandfec: in-band FEC встраивает сжатую копию предыдущего кадра, поэтому
    //   одиночная потеря пакета восстанавливается без заикания (главный выигрыш на
    //   нестабильном Wi-Fi/мобиле).
    // - packetlossperc: подсказка кодеку, сколько избыточности FEC закладывать.
    // - usedtx: во время тишины кодер почти не шлёт данные — экономит трафик; у нас
    //   уже есть собственный VAD, так что паузы безопасны.
    // - application=voip и frameDuration=20мс — профиль и размер кадра для речи
    //   (20мс снижает накладные расходы пакетов; это же значение по умолчанию).
    //
    // Неизвестные ключи браузер игнорирует при нормализации конфигурации, поэтому
    // на старых движках это безопасно — FEC просто не включится.
    let opus = Object::new();
    let _ = Reflect::set(
        &opus,
        &JsValue::from_str("useinbandfec"),
        &JsValue::from_bool(true),
    );
    let _ = Reflect::set(
        &opus,
        &JsValue::from_str("usedtx"),
        &JsValue::from_bool(true),
    );
    let _ = Reflect::set(
        &opus,
        &JsValue::from_str("application"),
        &JsValue::from_str("voip"),
    );
    let _ = Reflect::set(
        &opus,
        &JsValue::from_str("frameDuration"),
        &JsValue::from_f64(20_000.0),
    );
    let _ = Reflect::set(
        &opus,
        &JsValue::from_str("packetlossperc"),
        &JsValue::from_f64(10.0),
    );
    let _ = Reflect::set(&object, &JsValue::from_str("opus"), &opus);

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
    context: WorkletMessageContext<'_>,
) -> Result<(), MicrophoneError> {
    let diagnostics_enabled = microphone_diagnostics_enabled();
    let started_at = diagnostics_enabled.then(Instant::now);
    let message = WorkletMessage::from_message(data)?;
    let chunk = match message {
        WorkletMessage::Chunk(chunk) => chunk,
        WorkletMessage::ProcessorProfile(profile) => {
            context.diagnostics.record_processor_profile(profile);
            return Ok(());
        }
    };
    if chunk.samples.is_empty() {
        return Ok(());
    }
    let parse_elapsed_us = elapsed_us_since(&started_at);

    let duration_us = duration_us(chunk.samples.len(), context.sample_rate_hz);
    context
        .diagnostics
        .warn_if_delivery_is_late(WorkletDeliveryTiming {
            timestamp_us: chunk.timestamp_us,
            duration_us,
        });
    let mut samples = chunk.samples;
    apply_input_gain(&mut samples, context.input_gain);
    let rms = rms_level(&samples);
    let previous_active = context.detector.borrow().is_active();
    let active = context.detector.borrow_mut().update(rms, duration_us);
    if active != previous_active {
        debug!(
            rms,
            active,
            threshold = detector_threshold(context.detector),
            timestamp_us = chunk.timestamp_us,
            "microphone voice activation changed"
        );
    }
    (context.callbacks.on_level)(MicrophoneLevel {
        rms,
        active,
        threshold: detector_threshold(context.detector),
        timestamp_us: chunk.timestamp_us,
    });
    let level_elapsed_us = elapsed_us_since(&started_at).saturating_sub(parse_elapsed_us);

    if !active {
        if diagnostics_enabled {
            context
                .diagnostics
                .record_processing(WorkletProcessingTiming {
                    timestamp_us: chunk.timestamp_us,
                    duration_us,
                    active,
                    total_elapsed_us: elapsed_us_since(&started_at),
                    parse_elapsed_us,
                    level_elapsed_us,
                    audio_data_elapsed_us: 0,
                    encode_elapsed_us: 0,
                    close_elapsed_us: 0,
                    queue_size: context.encoder.encode_queue_size(),
                });
        }
        return Ok(());
    }

    let audio_data_started_at = diagnostics_enabled.then(Instant::now);
    let audio = audio_data_from_samples(&samples, context.sample_rate_hz, chunk.timestamp_us)?;
    let audio_data_elapsed_us = elapsed_us_since(&audio_data_started_at);
    let encode_started_at = diagnostics_enabled.then(Instant::now);
    let encode_result = context.encoder.encode(audio.as_ref());
    let encode_elapsed_us = elapsed_us_since(&encode_started_at);
    let close_started_at = diagnostics_enabled.then(Instant::now);
    let close_result = audio.close();
    let close_elapsed_us = elapsed_us_since(&close_started_at);
    let queue_size = context.encoder.encode_queue_size();
    if diagnostics_enabled && encode_result.is_ok() {
        context
            .encoder_diagnostics
            .record_input(EncoderInputTiming { duration_us });
    }
    if diagnostics_enabled {
        context
            .diagnostics
            .record_processing(WorkletProcessingTiming {
                timestamp_us: chunk.timestamp_us,
                duration_us,
                active,
                total_elapsed_us: elapsed_us_since(&started_at),
                parse_elapsed_us,
                level_elapsed_us,
                audio_data_elapsed_us,
                encode_elapsed_us,
                close_elapsed_us,
                queue_size,
            });
    }
    if queue_size >= MICROPHONE_ENCODER_QUEUE_WARN_FRAMES
        && context.diagnostics.should_emit_encoder_queue_warning()
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

struct WorkletMessageContext<'a> {
    encoder: &'a AudioEncoder,
    detector: &'a Rc<RefCell<VoiceActivityDetector>>,
    callbacks: &'a MicrophoneCallbacks,
    sample_rate_hz: u32,
    input_gain: f32,
    diagnostics: &'a MicrophoneWorkletDiagnostics,
    encoder_diagnostics: &'a MicrophoneEncoderOutputDiagnostics,
}

enum WorkletMessage {
    Chunk(PcmChunk),
    ProcessorProfile(WorkletProcessorProfileTiming),
}

impl WorkletMessage {
    fn from_message(data: JsValue) -> Result<Self, MicrophoneError> {
        if Reflect::get(&data, &JsValue::from_str("kind"))
            .map_err(microphone_error)?
            .as_string()
            .as_deref()
            == Some("profile")
        {
            return Ok(Self::ProcessorProfile(WorkletProcessorProfileTiming {
                process_count: profile_u64(&data, "processCount")?,
                chunk_count: profile_u64(&data, "chunkCount")?,
                input_empty_count: profile_u64(&data, "inputEmptyCount")?,
                frames: profile_u64(&data, "frames")?,
                window_ms: profile_u64(&data, "windowMs")?,
                total_elapsed_us: u128::from(profile_u64(&data, "totalElapsedUs")?),
                clear_elapsed_us: u128::from(profile_u64(&data, "clearElapsedUs")?),
                copy_elapsed_us: u128::from(profile_u64(&data, "copyElapsedUs")?),
                post_elapsed_us: u128::from(profile_u64(&data, "postElapsedUs")?),
                allocate_elapsed_us: u128::from(profile_u64(&data, "allocateElapsedUs")?),
                max_process_us: u128::from(profile_u64(&data, "maxProcessUs")?),
                max_channels: profile_u64(&data, "maxChannels")?.min(u64::from(u32::MAX)) as u32,
            }));
        }

        PcmChunk::from_message(data).map(Self::Chunk)
    }
}

struct PcmChunk {
    samples: Vec<f32>,
    timestamp_us: u64,
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

fn profile_u64(data: &JsValue, name: &str) -> Result<u64, MicrophoneError> {
    Ok(Reflect::get(data, &JsValue::from_str(name))
        .map_err(microphone_error)?
        .as_f64()
        .unwrap_or(0.0)
        .max(0.0) as u64)
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

fn detector_threshold(detector: &Rc<RefCell<VoiceActivityDetector>>) -> f32 {
    detector.borrow().config().vad_threshold
}

fn elapsed_us_since(started_at: &Option<Instant>) -> u128 {
    started_at
        .as_ref()
        .map(|started_at| started_at.elapsed().as_micros())
        .unwrap_or_default()
}

fn microphone_error(error: JsValue) -> MicrophoneError {
    MicrophoneError::new(js_error_message(error))
}

fn set_property(object: &Object, name: &str, value: &JsValue) -> Result<(), MicrophoneError> {
    Reflect::set(object, &JsValue::from_str(name), value)
        .map(|_| ())
        .map_err(microphone_error)
}
