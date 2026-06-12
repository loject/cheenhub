//! Browser microphone backend orchestration.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::{info, warn};
use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;
use js_sys::{Function, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioContext, AudioNode, AudioWorkletNode, GainNode, MediaStream, MediaStreamAudioSourceNode,
    MessageEvent, MessagePort,
};

use super::backend::{
    MicrophoneBackend, MicrophoneCallbacks, MicrophoneCodec, MicrophoneConfig, MicrophoneError,
    MicrophoneSession,
};
use super::browser_bindings::{AudioEncoder, EncodedAudioChunk};
use super::browser_capture::{
    first_audio_track, log_selected_audio_track, request_microphone_stream,
};
use super::browser_encoding::{
    close_encoder_lossy, create_encoder, encoder_config, microphone_message_closure,
};
use super::browser_errors::js_error_message;
use super::browser_worklet::{
    connect_capture_graph, create_audio_context, create_worklet_node, load_worklet_module,
    worklet_chunk_ms,
};

/// Browser microphone implementation backed by AudioWorklet and WebCodecs.
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
    encoder: AudioEncoder,
    context: AudioContext,
    track: web_sys::MediaStreamTrack,
    source: MediaStreamAudioSourceNode,
    worklet: AudioWorkletNode,
    silent_gain: GainNode,
    port: MessagePort,
    closed: Rc<Cell<bool>>,
    _message_closure: Closure<dyn FnMut(MessageEvent)>,
    _processor_error_closure: Closure<dyn FnMut(JsValue)>,
    _output_closure: Closure<dyn FnMut(EncodedAudioChunk)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
    bitrate_bps: Rc<Cell<u32>>,
}

impl MicrophoneSession for BrowserMicrophoneSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        let encoder = self.encoder.clone();
        let context = self.context.clone();
        let track = self.track.clone();
        let source = self.source.clone();
        let worklet = self.worklet.clone();
        let silent_gain = self.silent_gain.clone();
        let port = self.port.clone();
        let closed = self.closed.clone();
        async move {
            if closed.replace(true) {
                return Ok(());
            }

            port.set_onmessage(None);
            port.close();
            worklet.set_onprocessorerror(None);
            disconnect_audio_node(source.as_ref());
            disconnect_audio_node(worklet.as_ref());
            disconnect_audio_node(silent_gain.as_ref());
            track.stop();
            encoder.close().map_err(microphone_error)?;
            JsFuture::from(context.close().map_err(microphone_error)?)
                .await
                .map_err(microphone_error)?;
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
    validate_config(&config)?;
    ensure_browser_microphone_support()?;

    let context = create_audio_context(config.sample_rate_hz)?;
    let sample_rate_hz = context.sample_rate().round().max(1.0) as u32;
    if sample_rate_hz != config.sample_rate_hz {
        close_context_lossy(&context).await;
        return Err(MicrophoneError::new(format!(
            "Браузер создал аудиоконтекст {sample_rate_hz} Гц вместо необходимых {} Гц.",
            config.sample_rate_hz
        )));
    }
    if let Err(error) = load_worklet_module(&context).await {
        close_context_lossy(&context).await;
        return Err(error);
    }

    let encoder_config = encoder_config(sample_rate_hz, config.channels, config.bitrate_bps);
    verify_encoder_support(&context, &encoder_config).await?;
    let stream = request_stream_or_close_context(&context, config.clone()).await?;
    let track = first_track_or_cleanup(&stream, &context).await?;
    log_selected_audio_track(&track, config.device_id.as_deref());

    let encoder = match create_encoder(&encoder_config, &config, callbacks.on_frame.clone()) {
        Ok(encoder) => encoder,
        Err(error) => {
            stop_media_stream(&stream);
            close_context_lossy(&context).await;
            return Err(error);
        }
    };

    let source = match context.create_media_stream_source(&stream) {
        Ok(source) => source,
        Err(error) => {
            stop_media_stream(&stream);
            close_encoder_lossy(&encoder.encoder);
            close_context_lossy(&context).await;
            return Err(microphone_error(error));
        }
    };
    let worklet = match create_worklet_node(&context, sample_rate_hz) {
        Ok(worklet) => worklet,
        Err(error) => {
            stop_media_stream(&stream);
            close_encoder_lossy(&encoder.encoder);
            close_context_lossy(&context).await;
            return Err(error);
        }
    };
    let silent_gain = match context.create_gain() {
        Ok(gain) => gain,
        Err(error) => {
            stop_media_stream(&stream);
            close_encoder_lossy(&encoder.encoder);
            close_context_lossy(&context).await;
            return Err(microphone_error(error));
        }
    };
    silent_gain.gain().set_value(0.0);

    let closed = Rc::new(Cell::new(false));
    let port = match worklet.port() {
        Ok(port) => port,
        Err(error) => {
            stop_media_stream(&stream);
            close_encoder_lossy(&encoder.encoder);
            close_context_lossy(&context).await;
            return Err(microphone_error(error));
        }
    };
    let message_closure = microphone_message_closure(
        encoder.encoder.clone(),
        callbacks,
        config.clone(),
        sample_rate_hz,
        closed.clone(),
    );
    port.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));
    port.start();

    let processor_error_closure = Closure::wrap(Box::new(move |error: JsValue| {
        warn!(
            error = %js_error_message(error),
            "microphone audio worklet processor failed"
        );
    }) as Box<dyn FnMut(JsValue)>);
    worklet.set_onprocessorerror(Some(processor_error_closure.as_ref().unchecked_ref()));

    if let Err(error) = connect_capture_graph(&source, &worklet, &silent_gain, &context) {
        stop_media_stream(&stream);
        close_encoder_lossy(&encoder.encoder);
        close_context_lossy(&context).await;
        return Err(error);
    }
    if let Err(error) = JsFuture::from(context.resume().map_err(microphone_error)?).await {
        stop_media_stream(&stream);
        close_encoder_lossy(&encoder.encoder);
        close_context_lossy(&context).await;
        return Err(microphone_error(error));
    }

    info!(
        sample_rate_hz,
        chunk_ms = worklet_chunk_ms(),
        "browser microphone audio worklet capture started"
    );
    Ok(Rc::new(BrowserMicrophoneSession {
        encoder: encoder.encoder,
        context,
        track,
        source,
        worklet,
        silent_gain,
        port,
        closed,
        _message_closure: message_closure,
        _processor_error_closure: processor_error_closure,
        _output_closure: encoder.output_closure,
        _error_closure: encoder.error_closure,
        bitrate_bps: Rc::new(Cell::new(config.bitrate_bps)),
    }))
}

fn validate_config(config: &MicrophoneConfig) -> Result<(), MicrophoneError> {
    if config.codec != MicrophoneCodec::Opus {
        return Err(MicrophoneError::new("Поддерживается только Opus микрофон."));
    }
    if config.channels != 1 {
        return Err(MicrophoneError::new("Поддерживается только моно микрофон."));
    }
    Ok(())
}

async fn verify_encoder_support(
    context: &AudioContext,
    encoder_config: &JsValue,
) -> Result<(), MicrophoneError> {
    let support = match JsFuture::from(AudioEncoder::is_config_supported(encoder_config)).await {
        Ok(support) => support,
        Err(error) => {
            close_context_lossy(context).await;
            return Err(microphone_error(error));
        }
    };
    let supported = match Reflect::get(&support, &JsValue::from_str("supported")) {
        Ok(supported) => supported.as_bool().unwrap_or(false),
        Err(error) => {
            close_context_lossy(context).await;
            return Err(microphone_error(error));
        }
    };
    if !supported {
        close_context_lossy(context).await;
        return Err(MicrophoneError::new(
            "Браузер не поддерживает кодирование микрофона в Opus.",
        ));
    }
    Ok(())
}

async fn request_stream_or_close_context(
    context: &AudioContext,
    config: MicrophoneConfig,
) -> Result<MediaStream, MicrophoneError> {
    match request_microphone_stream(config).await {
        Ok(stream) => Ok(stream),
        Err(error) => {
            close_context_lossy(context).await;
            Err(error)
        }
    }
}

async fn first_track_or_cleanup(
    stream: &MediaStream,
    context: &AudioContext,
) -> Result<web_sys::MediaStreamTrack, MicrophoneError> {
    match first_audio_track(stream) {
        Ok(track) => Ok(track),
        Err(error) => {
            stop_media_stream(stream);
            close_context_lossy(context).await;
            Err(error)
        }
    }
}

fn ensure_browser_microphone_support() -> Result<(), MicrophoneError> {
    if !global_constructor_available("AudioContext")
        && !global_constructor_available("webkitAudioContext")
    {
        return Err(MicrophoneError::new(
            "Браузер не поддерживает AudioContext для захвата микрофона.",
        ));
    }
    if !global_constructor_available("AudioWorkletNode") {
        return Err(MicrophoneError::new(
            "Браузер не поддерживает AudioWorklet для захвата микрофона.",
        ));
    }
    if !global_constructor_available("AudioEncoder") {
        return Err(MicrophoneError::new(
            "Браузер не поддерживает кодирование микрофона через WebCodecs.",
        ));
    }
    Ok(())
}

fn global_constructor_available(name: &str) -> bool {
    Reflect::get(&js_sys::global(), &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .is_some()
}

fn stop_media_stream(stream: &MediaStream) {
    let tracks = stream.get_audio_tracks();
    for i in 0..tracks.length() {
        if let Ok(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
            track.stop();
        }
    }
}

async fn close_context_lossy(context: &AudioContext) {
    if let Ok(promise) = context.close() {
        let _ = JsFuture::from(promise).await;
    }
}

fn disconnect_audio_node(node: &AudioNode) {
    let _ = node.disconnect();
}

fn microphone_error(error: JsValue) -> MicrophoneError {
    MicrophoneError::new(js_error_message(error))
}
