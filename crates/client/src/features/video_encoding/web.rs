//! Браузерная реализация кодирования видео через WebCodecs.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use futures_util::FutureExt;
use js_sys::{Function, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use super::backend::{
    EncodedVideoFrame, EncodedVideoFrameCallback, VideoCodec, VideoEncoderConfig,
    VideoEncoderDescriptor, VideoEncodingAccelerator, VideoEncodingAcceleratorKind,
    VideoEncodingError, VideoEncodingManager, VideoFrameEncoder,
};
use super::web_frame_source::BrowserVideoFrameReader;

const VP9_WEB_CODECS_CODEC: &str = "vp09.00.10.08";
const WEB_CODECS_ENCODER_ID: &str = "webcodecs-vp9";

/// Менеджер браузерных кодировщиков видео.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BrowserVideoEncodingManager;

impl BrowserVideoEncodingManager {
    /// Возвращает, доступен ли browser pipeline захвата и кодирования видео.
    pub(crate) fn browser_capture_pipeline_available(&self) -> bool {
        WebCodecsVideoAccelerator.constructors_available()
            && BrowserVideoFrameReader::capture_available()
    }
}

impl VideoEncodingManager for BrowserVideoEncodingManager {
    type Encoder = BrowserVideoEncoder;
    type InputFrame = BrowserVideoFrame;

    fn available_accelerators(
        &self,
        config: VideoEncoderConfig,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<Vec<VideoEncoderDescriptor>, VideoEncodingError>,
    > {
        let accelerator = WebCodecsVideoAccelerator;
        async move {
            if !accelerator.constructors_available() {
                return Ok(Vec::new());
            }
            if !webcodecs_config_supported(&config).await? {
                return Ok(Vec::new());
            }

            Ok(vec![accelerator.descriptor()])
        }
        .boxed_local()
    }

    fn create_encoder(
        &self,
        kind: VideoEncodingAcceleratorKind,
        config: VideoEncoderConfig,
        on_frame: EncodedVideoFrameCallback,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<BrowserVideoEncoder, VideoEncodingError>,
    > {
        match kind {
            VideoEncodingAcceleratorKind::WebCodecs => {
                WebCodecsVideoAccelerator.create_encoder(config, on_frame)
            }
            VideoEncodingAcceleratorKind::WasmCpu | VideoEncodingAcceleratorKind::Native => {
                async move {
                    Err(VideoEncodingError::unsupported(format!(
                        "Кодировщик {kind:?} недоступен в браузере."
                    )))
                }
                .boxed_local()
            }
        }
    }
}

/// WebCodecs-реализация браузерного кодировщика видео.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WebCodecsVideoAccelerator;

impl WebCodecsVideoAccelerator {
    fn constructors_available(&self) -> bool {
        global_constructor_available("VideoEncoder") && global_constructor_available("VideoFrame")
    }
}

impl VideoEncodingAccelerator for WebCodecsVideoAccelerator {
    type Encoder = BrowserVideoEncoder;
    type InputFrame = BrowserVideoFrame;

    fn descriptor(&self) -> VideoEncoderDescriptor {
        VideoEncoderDescriptor {
            id: WEB_CODECS_ENCODER_ID.to_owned(),
            label: "WebCodecs VP9".to_owned(),
            kind: VideoEncodingAcceleratorKind::WebCodecs,
            codecs: vec![VideoCodec::Vp9],
        }
    }

    fn create_encoder(
        &self,
        config: VideoEncoderConfig,
        on_frame: EncodedVideoFrameCallback,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<BrowserVideoEncoder, VideoEncodingError>,
    > {
        async move {
            if !WebCodecsVideoAccelerator.constructors_available() {
                return Err(VideoEncodingError::unsupported(
                    "Браузер не поддерживает видеокодирование через WebCodecs.",
                ));
            }
            if !webcodecs_config_supported(&config).await? {
                return Err(VideoEncodingError::unsupported(
                    "Браузер не поддерживает запрошенную конфигурацию VP9.",
                ));
            }

            create_webcodecs_encoder(config, on_frame)
        }
        .boxed_local()
    }
}

/// Активный браузерный кодировщик видео.
pub(crate) struct BrowserVideoEncoder {
    handle: BrowserVideoEncoderHandle,
    _output_closure: Closure<dyn FnMut(EncodedVideoChunk)>,
    _error_closure: Closure<dyn FnMut(JsValue)>,
}

impl BrowserVideoEncoder {
    /// Возвращает легкий cloneable handle кодировщика.
    pub(crate) fn handle(&self) -> BrowserVideoEncoderHandle {
        self.handle.clone()
    }
}

impl VideoFrameEncoder for BrowserVideoEncoder {
    type InputFrame = BrowserVideoFrame;

    fn encode(&self, frame: &Self::InputFrame, key_frame: bool) -> Result<(), VideoEncodingError> {
        self.handle.encode(frame, key_frame)
    }

    fn close(&self) -> Result<(), VideoEncodingError> {
        self.handle.close()
    }
}

/// Cloneable handle активного браузерного кодировщика.
#[derive(Clone)]
pub(crate) struct BrowserVideoEncoderHandle {
    encoder: VideoEncoder,
}

impl VideoFrameEncoder for BrowserVideoEncoderHandle {
    type InputFrame = BrowserVideoFrame;

    fn encode(&self, frame: &Self::InputFrame, key_frame: bool) -> Result<(), VideoEncodingError> {
        if !key_frame {
            return self.encoder.encode(&frame.frame).map_err(video_error);
        }

        let options = Object::new();
        set_property(&options, "keyFrame", &JsValue::TRUE);
        let options: JsValue = options.into();
        self.encoder
            .encode_with_options(&frame.frame, &options)
            .map_err(video_error)
    }

    fn close(&self) -> Result<(), VideoEncodingError> {
        self.encoder.close().map_err(video_error)
    }
}

/// Браузерный входной видео-кадр.
pub(crate) struct BrowserVideoFrame {
    frame: VideoFrame,
}

impl BrowserVideoFrame {
    /// Создает wrapper из значения, полученного из `ReadableStream`.
    pub(crate) fn from_js_value(value: &JsValue) -> Self {
        Self {
            frame: value.unchecked_ref::<VideoFrame>().clone(),
        }
    }

    /// Закрывает кадр и освобождает браузерный ресурс.
    pub(crate) fn close(&self) {
        let _ = self.frame.close();
    }

    /// Возвращает временную метку входного кадра в микросекундах.
    pub(crate) fn timestamp_us(&self) -> u64 {
        self.frame.timestamp().max(0.0).round() as u64
    }
}

fn create_webcodecs_encoder(
    config: VideoEncoderConfig,
    on_frame: EncodedVideoFrameCallback,
) -> Result<BrowserVideoEncoder, VideoEncodingError> {
    let sequence = Rc::new(Cell::new(0_u64));
    let output_sequence = sequence.clone();
    let codec = config.codec;
    let width = config.width;
    let height = config.height;
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
        on_frame(EncodedVideoFrame {
            sequence,
            timestamp_us: chunk.timestamp().max(0.0) as u64,
            duration_us: chunk.duration().unwrap_or(0.0).max(0.0) as u32,
            codec,
            key_frame: chunk.chunk_type() == "key",
            width,
            height,
            bytes,
        });
    }) as Box<dyn FnMut(EncodedVideoChunk)>);
    let error_closure = Closure::wrap(Box::new(move |error: JsValue| {
        warn!(
            error = %js_error_message(error),
            codec = ?codec,
            accelerator = "webcodecs",
            "browser video encoder failed"
        );
    }) as Box<dyn FnMut(JsValue)>);
    let init = encoder_init(
        error_closure.as_ref().unchecked_ref(),
        output_closure.as_ref().unchecked_ref(),
    )?;
    let encoder_config = webcodecs_encoder_config(&config);
    let encoder = VideoEncoder::new(&init).map_err(video_error)?;
    encoder.configure(&encoder_config).map_err(video_error)?;
    debug!(
        codec = ?config.codec,
        width = config.width,
        height = config.height,
        frame_rate = config.frame_rate,
        bitrate_bps = config.bitrate_bps,
        accelerator = "webcodecs",
        "browser video encoder configured"
    );

    Ok(BrowserVideoEncoder {
        handle: BrowserVideoEncoderHandle { encoder },
        _output_closure: output_closure,
        _error_closure: error_closure,
    })
}

async fn webcodecs_config_supported(
    config: &VideoEncoderConfig,
) -> Result<bool, VideoEncodingError> {
    let encoder_config = webcodecs_encoder_config(config);
    let support = JsFuture::from(VideoEncoder::is_config_supported(&encoder_config))
        .await
        .map_err(video_error)?;
    Reflect::get(&support, &JsValue::from_str("supported"))
        .map_err(video_error)
        .map(|value| value.as_bool().unwrap_or(false))
}

fn webcodecs_encoder_config(config: &VideoEncoderConfig) -> JsValue {
    let object = Object::new();
    set_property(
        &object,
        "codec",
        &JsValue::from_str(match config.codec {
            VideoCodec::Vp9 => VP9_WEB_CODECS_CODEC,
        }),
    );
    set_property(
        &object,
        "width",
        &JsValue::from_f64(f64::from(config.width)),
    );
    set_property(
        &object,
        "height",
        &JsValue::from_f64(f64::from(config.height)),
    );
    set_property(
        &object,
        "framerate",
        &JsValue::from_f64(f64::from(config.frame_rate)),
    );
    set_property(
        &object,
        "bitrate",
        &JsValue::from_f64(f64::from(config.bitrate_bps)),
    );
    object.into()
}

fn encoder_init(error: &Function, output: &Function) -> Result<JsValue, VideoEncodingError> {
    let object = Object::new();
    Reflect::set(&object, &JsValue::from_str("error"), error).map_err(video_error)?;
    Reflect::set(&object, &JsValue::from_str("output"), output).map_err(video_error)?;
    Ok(object.into())
}

pub(super) fn global_constructor_available(name: &str) -> bool {
    Reflect::get(&js_sys::global(), &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .is_some()
}

fn set_property(object: &Object, name: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(name), value);
}

fn video_error(error: JsValue) -> VideoEncodingError {
    VideoEncodingError::unavailable(js_error_message(error))
}

fn js_error_message(error: JsValue) -> String {
    error
        .dyn_ref::<js_sys::Error>()
        .map(js_sys::Error::message)
        .and_then(|message| message.as_string())
        .filter(|message| !message.is_empty())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "Неизвестная ошибка браузера.".to_owned())
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = VideoEncoder)]
    #[derive(Clone)]
    type VideoEncoder;

    #[wasm_bindgen(constructor, catch, js_class = VideoEncoder)]
    fn new(init: &JsValue) -> Result<VideoEncoder, JsValue>;

    #[wasm_bindgen(static_method_of = VideoEncoder, js_name = isConfigSupported)]
    fn is_config_supported(config: &JsValue) -> Promise;

    #[wasm_bindgen(method, catch, js_name = configure)]
    fn configure(this: &VideoEncoder, config: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = encode)]
    fn encode(this: &VideoEncoder, frame: &VideoFrame) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = encode)]
    fn encode_with_options(
        this: &VideoEncoder,
        frame: &VideoFrame,
        options: &JsValue,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = close)]
    fn close(this: &VideoEncoder) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = EncodedVideoChunk)]
    #[derive(Clone)]
    type EncodedVideoChunk;

    #[wasm_bindgen(method, getter, js_name = byteLength)]
    fn byte_length(this: &EncodedVideoChunk) -> u32;

    #[wasm_bindgen(method, getter)]
    fn timestamp(this: &EncodedVideoChunk) -> f64;

    #[wasm_bindgen(method, getter)]
    fn duration(this: &EncodedVideoChunk) -> Option<f64>;

    #[wasm_bindgen(method, getter, js_name = type)]
    fn chunk_type(this: &EncodedVideoChunk) -> String;

    #[wasm_bindgen(method, catch, js_name = copyTo)]
    fn copy_to(this: &EncodedVideoChunk, destination: &Uint8Array) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = VideoFrame)]
    #[derive(Clone)]
    type VideoFrame;

    #[wasm_bindgen(method, catch, js_name = close)]
    fn close(this: &VideoFrame) -> Result<(), JsValue>;

    #[wasm_bindgen(method, getter)]
    fn timestamp(this: &VideoFrame) -> f64;

}
