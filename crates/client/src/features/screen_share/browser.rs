//! Browser-backend демонстрации экрана.

use std::cell::Cell;
use std::rc::Rc;

use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;
use js_sys::{Function, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use super::backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCodec,
    ScreenShareConfig, ScreenShareError, ScreenShareSession,
};
use super::browser_bindings::{
    EncodedVideoChunk, MediaStreamTrackProcessor, VideoEncoder, VideoFrame,
};
use super::browser_capture::{
    first_video_track, log_selected_video_track, request_screen_stream, video_track_settings,
};
use super::browser_errors::js_error_message;

const KEY_FRAME_INTERVAL_SECONDS: u32 = 2;

/// Реализация браузерной демонстрации экрана на основе `getDisplayMedia` и WebCodecs.
pub(crate) struct BrowserScreenShareBackend;

impl ScreenShareBackend for BrowserScreenShareBackend {
    fn start(
        &self,
        config: ScreenShareConfig,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        async move { start_browser_session(config, callbacks).await }.boxed_local()
    }
}

struct BrowserScreenShareSession {
    encoder: JsValue,
    track: web_sys::MediaStreamTrack,
    closed: Rc<Cell<bool>>,
    _output_closure: Closure<dyn FnMut(EncodedVideoChunk)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
}

impl ScreenShareSession for BrowserScreenShareSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>> {
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
}

async fn start_browser_session(
    config: ScreenShareConfig,
    callbacks: ScreenShareCallbacks,
) -> Result<Rc<dyn ScreenShareSession>, ScreenShareError> {
    if config.codec != ScreenShareCodec::Vp9 {
        return Err(ScreenShareError::new(
            "Поддерживается только VP9 демонстрация экрана.",
        ));
    }

    let stream = request_screen_stream().await?;
    let track = first_video_track(&stream)?;
    log_selected_video_track(&track);
    let settings = video_track_settings(&track);
    let width = settings.width.unwrap_or(config.fallback_width).max(1);
    let height = settings.height.unwrap_or(config.fallback_height).max(1);
    let frame_rate = settings.frame_rate.unwrap_or(config.frame_rate).max(1);
    let encoder_config = encoder_config(&config, width, height, frame_rate);
    let support = JsFuture::from(VideoEncoder::is_config_supported(&encoder_config))
        .await
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;
    let supported = Reflect::get(&support, &JsValue::from_str("supported"))
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?
        .as_bool()
        .unwrap_or(false);
    if !supported {
        track.stop();
        return Err(ScreenShareError::new(
            "Браузер не поддерживает кодирование демонстрации экрана в VP9.",
        ));
    }

    let sequence = Rc::new(Cell::new(0_u64));
    let output_on_frame = callbacks.on_frame.clone();
    let output_sequence = sequence.clone();
    let output_closure = Closure::wrap(Box::new(move |chunk: EncodedVideoChunk| {
        let byte_length = chunk.byte_length();
        let destination = Uint8Array::new_with_length(byte_length);
        if chunk.copy_to(&destination).is_err() {
            return;
        }
        let mut bytes = vec![0; byte_length as usize];
        destination.copy_to(&mut bytes);
        let sequence = output_sequence.get();
        output_sequence.set(sequence.saturating_add(1));
        let key_frame = chunk.chunk_type() == "key";
        output_on_frame(EncodedScreenShareFrame {
            sequence,
            timestamp_us: chunk.timestamp().max(0.0) as u64,
            duration_us: chunk.duration().unwrap_or(0.0).max(0.0) as u32,
            codec: ScreenShareCodec::Vp9,
            key_frame,
            width,
            height,
            bytes,
        });
    }) as Box<dyn FnMut(EncodedVideoChunk)>);
    let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
        console_warn(&format!(
            "browser screen sharing encoder failed: {}",
            js_error_message(error)
        ));
    }) as Box<dyn FnMut(JsValue)>);
    let encoder_init = encoder_init(
        error_closure.as_ref().unchecked_ref(),
        output_closure.as_ref().unchecked_ref(),
    )?;
    let encoder = VideoEncoder::new(&encoder_init).map_err(|error| {
        track.stop();
        ScreenShareError::new(js_error_message(error))
    })?;
    encoder.configure(&encoder_config).map_err(|error| {
        track.stop();
        ScreenShareError::new(js_error_message(error))
    })?;
    let encoder: JsValue = encoder.into();

    let closed = Rc::new(Cell::new(false));
    let key_frame_interval_frames = frame_rate.saturating_mul(KEY_FRAME_INTERVAL_SECONDS).max(1);
    spawn_video_reader(
        track.clone(),
        encoder.clone(),
        closed.clone(),
        callbacks,
        key_frame_interval_frames,
    );

    Ok(Rc::new(BrowserScreenShareSession {
        encoder,
        track,
        closed,
        _output_closure: output_closure,
        _error_closure: error_closure,
    }))
}

fn encoder_config(config: &ScreenShareConfig, width: u32, height: u32, frame_rate: u32) -> JsValue {
    let object = Object::new();
    set_property(&object, "codec", &JsValue::from_str("vp09.00.10.08"));
    set_property(&object, "width", &JsValue::from_f64(f64::from(width)));
    set_property(&object, "height", &JsValue::from_f64(f64::from(height)));
    set_property(
        &object,
        "framerate",
        &JsValue::from_f64(f64::from(frame_rate)),
    );
    set_property(
        &object,
        "bitrate",
        &JsValue::from_f64(f64::from(config.bitrate_bps)),
    );
    object.into()
}

fn encoder_init(error: &Function, output: &Function) -> Result<JsValue, ScreenShareError> {
    let object = Object::new();
    Reflect::set(&object, &JsValue::from_str("error"), error)
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;
    Reflect::set(&object, &JsValue::from_str("output"), output)
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;
    Ok(object.into())
}

fn spawn_video_reader(
    track: web_sys::MediaStreamTrack,
    encoder: JsValue,
    closed: Rc<Cell<bool>>,
    callbacks: ScreenShareCallbacks,
    key_frame_interval_frames: u32,
) {
    spawn_local(async move {
        let frame_sequence = Rc::new(Cell::new(0_u64));
        let processor = match media_stream_track_processor(&track) {
            Ok(processor) => processor,
            Err(error) => {
                console_warn(&format!(
                    "failed to create screen sharing track processor: {error}"
                ));
                finish_browser_capture(&encoder, &closed, &callbacks);
                return;
            }
        };
        let reader = match stream_reader(&processor.readable()) {
            Ok(reader) => reader,
            Err(error) => {
                console_warn(&format!(
                    "failed to create screen sharing stream reader: {error}"
                ));
                finish_browser_capture(&encoder, &closed, &callbacks);
                return;
            }
        };

        while !closed.get() {
            let read = match read_stream_chunk(&reader).await {
                Ok(read) => read,
                Err(error) => {
                    console_warn(&format!("failed to read screen sharing frame: {error}"));
                    break;
                }
            };
            if read.done {
                break;
            }
            let frame = read.value.unchecked_ref::<VideoFrame>();
            let sequence = frame_sequence.get();
            frame_sequence.set(sequence.saturating_add(1));
            let key_frame = sequence.is_multiple_of(u64::from(key_frame_interval_frames));
            if encode_video_frame(&encoder, frame, key_frame).is_err() {
                close_video_frame(frame);
                break;
            }
            close_video_frame(frame);
        }

        finish_browser_capture(&encoder, &closed, &callbacks);
    });
}

fn finish_browser_capture(
    encoder: &JsValue,
    closed: &Rc<Cell<bool>>,
    callbacks: &ScreenShareCallbacks,
) {
    if closed.replace(true) {
        return;
    }
    if let Err(error) = close_encoder(encoder) {
        console_warn(&format!(
            "failed to close screen sharing encoder after capture ended: {error}"
        ));
    }
    (callbacks.on_ended)();
}

fn media_stream_track_processor(
    track: &web_sys::MediaStreamTrack,
) -> Result<MediaStreamTrackProcessor, ScreenShareError> {
    let init = Object::new();
    Reflect::set(&init, &JsValue::from_str("track"), track.as_ref())
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;
    MediaStreamTrackProcessor::new(&init.into())
        .map_err(|error| ScreenShareError::new(js_error_message(error)))
}

fn stream_reader(readable: &JsValue) -> Result<JsValue, ScreenShareError> {
    let get_reader = Reflect::get(readable, &JsValue::from_str("getReader"))
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?
        .dyn_into::<Function>()
        .map_err(|_| ScreenShareError::new("ReadableStream.getReader недоступен."))?;
    get_reader
        .call0(readable)
        .map_err(|error| ScreenShareError::new(js_error_message(error)))
}

struct StreamRead {
    done: bool,
    value: JsValue,
}

async fn read_stream_chunk(reader: &JsValue) -> Result<StreamRead, ScreenShareError> {
    let read = Reflect::get(reader, &JsValue::from_str("read"))
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?
        .dyn_into::<Function>()
        .map_err(|_| ScreenShareError::new("ReadableStream reader.read недоступен."))?;
    let promise = read
        .call0(reader)
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?
        .dyn_into::<Promise>()
        .map_err(|_| ScreenShareError::new("ReadableStream reader.read не вернул Promise."))?;
    let result = JsFuture::from(promise)
        .await
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;
    let done = Reflect::get(&result, &JsValue::from_str("done"))
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?
        .as_bool()
        .unwrap_or(false);
    let value = Reflect::get(&result, &JsValue::from_str("value"))
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;

    Ok(StreamRead { done, value })
}

fn encode_video_frame(
    encoder: &JsValue,
    frame: &VideoFrame,
    key_frame: bool,
) -> Result<(), JsValue> {
    let encoder = encoder.unchecked_ref::<VideoEncoder>();
    if !key_frame {
        return encoder.encode(frame);
    }

    let options = Object::new();
    set_property(&options, "keyFrame", &JsValue::TRUE);
    encoder.encode_with_options(frame, &options.into())
}

fn close_video_frame(frame: &VideoFrame) {
    let _ = frame.close();
}

fn close_encoder(encoder: &JsValue) -> Result<(), ScreenShareError> {
    encoder
        .unchecked_ref::<VideoEncoder>()
        .close()
        .map_err(|error| ScreenShareError::new(js_error_message(error)))
}

fn console_warn(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

fn set_property(object: &Object, name: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(name), value);
}
