//! Browser microphone backend.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::prelude::debug;
use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;
use js_sys::{Float32Array, Function, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use super::backend::{
    EncodedMicrophoneFrame, MicrophoneBackend, MicrophoneCallbacks, MicrophoneCodec,
    MicrophoneConfig, MicrophoneError, MicrophoneLevel, MicrophoneSession,
};
use super::browser_bindings::{
    AudioData, AudioEncoder, EncodedAudioChunk, MediaStreamTrackProcessor,
};
use super::browser_capture::{
    first_audio_track, log_selected_audio_track, request_microphone_stream,
};
use super::browser_errors::js_error_message;
use super::vad::{VoiceActivityDetector, rms_level};

/// Browser microphone implementation backed by getUserMedia and WebCodecs.
pub(crate) struct BrowserMicrophoneBackend;

impl MicrophoneBackend for BrowserMicrophoneBackend {
    fn start(
        &self,
        config: MicrophoneConfig,
        callbacks: MicrophoneCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn MicrophoneSession>, MicrophoneError>> {
        async move { start_browser_session(config, callbacks).await }.boxed_local()
    }
}

struct BrowserMicrophoneSession {
    encoder: JsValue,
    track: web_sys::MediaStreamTrack,
    closed: Rc<Cell<bool>>,
    _output_closure: Closure<dyn FnMut(EncodedAudioChunk)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
    bitrate_bps: Rc<Cell<u32>>,
}

impl MicrophoneSession for BrowserMicrophoneSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        let encoder = self.encoder.clone();
        let track = self.track.clone();
        let closed = self.closed.clone();
        async move {
            if closed.replace(true) {
                return Ok(());
            }
            track.stop();
            close_encoder(&encoder)?;
            Ok(())
        }
        .boxed_local()
    }

    fn set_bitrate_bps(
        &self,
        bitrate_bps: u32,
    ) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        self.bitrate_bps.set(bitrate_bps);
        async move { Ok(()) }.boxed_local()
    }
}

async fn start_browser_session(
    config: MicrophoneConfig,
    callbacks: MicrophoneCallbacks,
) -> Result<Rc<dyn MicrophoneSession>, MicrophoneError> {
    if config.codec != MicrophoneCodec::Opus {
        return Err(MicrophoneError::new("Поддерживается только Opus микрофон."));
    }
    if config.channels != 1 {
        return Err(MicrophoneError::new("Поддерживается только моно микрофон."));
    }

    let stream = request_microphone_stream(config.clone()).await?;
    let track = first_audio_track(&stream)?;
    log_selected_audio_track(&track, config.device_id.as_deref());
    let encoder_config = encoder_config(config.clone());
    let support = JsFuture::from(AudioEncoder::is_config_supported(&encoder_config))
        .await
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    let supported = Reflect::get(&support, &JsValue::from_str("supported"))
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?
        .as_bool()
        .unwrap_or(false);
    if !supported {
        return Err(MicrophoneError::new(
            "Браузер не поддерживает кодирование микрофона в Opus.",
        ));
    }

    let sequence = Rc::new(Cell::new(0_u64));
    let bitrate_bps = Rc::new(Cell::new(config.bitrate_bps));
    let output_on_frame = callbacks.on_frame.clone();
    let output_sequence = sequence.clone();
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
        output_on_frame(EncodedMicrophoneFrame {
            sequence,
            timestamp_us: chunk.timestamp().max(0.0) as u64,
            duration_us: chunk.duration().unwrap_or(0.0).max(0.0) as u32,
            codec: MicrophoneCodec::Opus,
            sample_rate_hz: config.sample_rate_hz,
            channels: config.channels,
            bytes,
        });
    }) as Box<dyn FnMut(EncodedAudioChunk)>);
    let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
        console_warn(&format!(
            "browser microphone encoder failed: {}",
            js_error_message(error)
        ));
    }) as Box<dyn FnMut(JsValue)>);
    let encoder_init = encoder_init(
        error_closure.as_ref().unchecked_ref(),
        output_closure.as_ref().unchecked_ref(),
    )?;
    let encoder = AudioEncoder::new(&encoder_init)
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    encoder
        .configure(&encoder_config)
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    let encoder: JsValue = encoder.into();

    let closed = Rc::new(Cell::new(false));
    spawn_audio_reader(
        track.clone(),
        encoder.clone(),
        closed.clone(),
        config,
        callbacks,
    );

    Ok(Rc::new(BrowserMicrophoneSession {
        encoder,
        track,
        closed,
        _output_closure: output_closure,
        _error_closure: error_closure,
        bitrate_bps,
    }))
}

fn encoder_config(config: MicrophoneConfig) -> JsValue {
    let object = Object::new();
    set_property(&object, "codec", &JsValue::from_str("opus"));
    set_property(
        &object,
        "sampleRate",
        &JsValue::from_f64(f64::from(config.sample_rate_hz)),
    );
    set_property(
        &object,
        "numberOfChannels",
        &JsValue::from_f64(f64::from(config.channels)),
    );
    set_property(
        &object,
        "bitrate",
        &JsValue::from_f64(f64::from(config.bitrate_bps)),
    );
    object.into()
}

fn encoder_init(error: &Function, output: &Function) -> Result<JsValue, MicrophoneError> {
    let object = Object::new();
    Reflect::set(&object, &JsValue::from_str("error"), error)
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    Reflect::set(&object, &JsValue::from_str("output"), output)
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    Ok(object.into())
}

fn spawn_audio_reader(
    track: web_sys::MediaStreamTrack,
    encoder: JsValue,
    closed: Rc<Cell<bool>>,
    config: MicrophoneConfig,
    callbacks: MicrophoneCallbacks,
) {
    spawn_local(async move {
        let detector = Rc::new(RefCell::new(VoiceActivityDetector::new(config)));
        let processor = match media_stream_track_processor(&track) {
            Ok(processor) => processor,
            Err(error) => {
                console_warn(&format!(
                    "failed to create microphone track processor: {error}"
                ));
                return;
            }
        };
        let reader = match stream_reader(&processor.readable()) {
            Ok(reader) => reader,
            Err(error) => {
                console_warn(&format!(
                    "failed to create microphone stream reader: {error}"
                ));
                return;
            }
        };

        while !closed.get() {
            let read = match read_stream_chunk(&reader).await {
                Ok(read) => read,
                Err(error) => {
                    console_warn(&format!("failed to read microphone frame: {error}"));
                    break;
                }
            };
            if read.done {
                break;
            }
            let should_encode = match voice_gate_allows_audio(&read.value, &detector, &callbacks) {
                Ok(should_encode) => should_encode,
                Err(error) => {
                    console_warn(&format!("failed to measure microphone level: {error}"));
                    false
                }
            };
            if !should_encode {
                close_audio_data(&read.value);
                continue;
            }
            if encode_audio_data(&encoder, &read.value).is_err() {
                break;
            }
            close_audio_data(&read.value);
        }
    });
}

fn voice_gate_allows_audio(
    value: &JsValue,
    detector: &Rc<RefCell<VoiceActivityDetector>>,
    callbacks: &MicrophoneCallbacks,
) -> Result<bool, MicrophoneError> {
    let audio = value.unchecked_ref::<AudioData>();
    let samples = audio_samples(audio)?;
    let rms = rms_level(&samples);
    let timestamp_us = audio.audio_data_timestamp().max(0.0) as u64;
    let duration_us = audio.audio_data_duration().unwrap_or(0.0).max(0.0) as u32;
    let previous_active = detector.borrow().is_active();
    let active = detector.borrow_mut().update(rms, duration_us);
    if active != previous_active {
        debug!(
            rms,
            active,
            threshold = detector_threshold(detector),
            timestamp_us,
            "microphone voice activation changed"
        );
    }
    (callbacks.on_level)(MicrophoneLevel {
        rms,
        active,
        threshold: detector_threshold(detector),
        timestamp_us,
    });

    Ok(active)
}

fn detector_threshold(detector: &Rc<RefCell<VoiceActivityDetector>>) -> f32 {
    detector.borrow().config().vad_threshold
}

fn audio_samples(audio: &AudioData) -> Result<Vec<f32>, MicrophoneError> {
    let frames = audio.audio_data_number_of_frames();
    let channels = audio.audio_data_number_of_channels().max(1);
    if frames == 0 {
        return Ok(Vec::new());
    }

    let mut samples = Vec::with_capacity(frames as usize * channels as usize);
    for channel in 0..channels {
        let channel_samples = Float32Array::new_with_length(frames);
        audio
            .audio_data_copy_to(&channel_samples, &copy_options(channel))
            .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
        samples.extend(channel_samples.to_vec());
    }

    Ok(samples)
}

fn copy_options(plane_index: u32) -> JsValue {
    let object = Object::new();
    set_property(&object, "format", &JsValue::from_str("f32-planar"));
    set_property(
        &object,
        "planeIndex",
        &JsValue::from_f64(f64::from(plane_index)),
    );
    object.into()
}

fn media_stream_track_processor(
    track: &web_sys::MediaStreamTrack,
) -> Result<MediaStreamTrackProcessor, MicrophoneError> {
    let init = Object::new();
    Reflect::set(&init, &JsValue::from_str("track"), track.as_ref())
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    MediaStreamTrackProcessor::new(&init.into())
        .map_err(|error| MicrophoneError::new(js_error_message(error)))
}

fn stream_reader(readable: &JsValue) -> Result<JsValue, MicrophoneError> {
    let get_reader = Reflect::get(readable, &JsValue::from_str("getReader"))
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?
        .dyn_into::<Function>()
        .map_err(|_| MicrophoneError::new("ReadableStream.getReader недоступен."))?;
    get_reader
        .call0(readable)
        .map_err(|error| MicrophoneError::new(js_error_message(error)))
}

struct StreamRead {
    done: bool,
    value: JsValue,
}

async fn read_stream_chunk(reader: &JsValue) -> Result<StreamRead, MicrophoneError> {
    let read = Reflect::get(reader, &JsValue::from_str("read"))
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?
        .dyn_into::<Function>()
        .map_err(|_| MicrophoneError::new("ReadableStream reader.read недоступен."))?;
    let promise = read
        .call0(reader)
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?
        .dyn_into::<Promise>()
        .map_err(|_| MicrophoneError::new("ReadableStream reader.read не вернул Promise."))?;
    let result = JsFuture::from(promise)
        .await
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    let done = Reflect::get(&result, &JsValue::from_str("done"))
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?
        .as_bool()
        .unwrap_or(false);
    let value = Reflect::get(&result, &JsValue::from_str("value"))
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;

    Ok(StreamRead { done, value })
}

fn close_audio_data(value: &JsValue) {
    if let Ok(close) = Reflect::get(value, &JsValue::from_str("close"))
        && let Ok(close) = close.dyn_into::<Function>()
    {
        let _ = close.call0(value);
    }
}

fn console_warn(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

fn close_encoder(encoder: &JsValue) -> Result<(), MicrophoneError> {
    let close = Reflect::get(encoder, &JsValue::from_str("close"))
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?
        .dyn_into::<Function>()
        .map_err(|_| MicrophoneError::new("AudioEncoder.close недоступен."))?;
    close
        .call0(encoder)
        .map(|_| ())
        .map_err(|error| MicrophoneError::new(js_error_message(error)))
}

fn encode_audio_data(encoder: &JsValue, value: &JsValue) -> Result<(), JsValue> {
    let encode = Reflect::get(encoder, &JsValue::from_str("encode"))?.dyn_into::<Function>()?;
    encode.call1(encoder, value).map(|_| ())
}

fn set_property(object: &Object, name: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(name), value);
}
