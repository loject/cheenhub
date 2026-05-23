//! Browser WebCodecs and stream bindings for screen sharing.

use js_sys::{Promise, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = VideoEncoder)]
    #[derive(Clone)]
    pub(super) type VideoEncoder;

    #[wasm_bindgen(constructor, catch, js_class = VideoEncoder)]
    pub(super) fn new(init: &JsValue) -> Result<VideoEncoder, JsValue>;

    #[wasm_bindgen(static_method_of = VideoEncoder, js_name = isConfigSupported)]
    pub(super) fn is_config_supported(config: &JsValue) -> Promise;

    #[wasm_bindgen(method, catch, js_name = configure)]
    pub(super) fn configure(this: &VideoEncoder, config: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = encode)]
    pub(super) fn encode(this: &VideoEncoder, frame: &VideoFrame) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = encode)]
    pub(super) fn encode_with_options(
        this: &VideoEncoder,
        frame: &VideoFrame,
        options: &JsValue,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = close)]
    pub(super) fn close(this: &VideoEncoder) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = EncodedVideoChunk)]
    #[derive(Clone)]
    pub(super) type EncodedVideoChunk;

    #[wasm_bindgen(method, getter, js_name = byteLength)]
    pub(super) fn byte_length(this: &EncodedVideoChunk) -> u32;

    #[wasm_bindgen(method, getter)]
    pub(super) fn timestamp(this: &EncodedVideoChunk) -> f64;

    #[wasm_bindgen(method, getter)]
    pub(super) fn duration(this: &EncodedVideoChunk) -> Option<f64>;

    #[wasm_bindgen(method, getter, js_name = type)]
    pub(super) fn chunk_type(this: &EncodedVideoChunk) -> String;

    #[wasm_bindgen(method, catch, js_name = copyTo)]
    pub(super) fn copy_to(
        this: &EncodedVideoChunk,
        destination: &Uint8Array,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = VideoFrame)]
    #[derive(Clone)]
    pub(super) type VideoFrame;

    #[wasm_bindgen(method, catch, js_name = close)]
    pub(super) fn close(this: &VideoFrame) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = MediaStreamTrackProcessor)]
    #[derive(Clone)]
    pub(super) type MediaStreamTrackProcessor;

    #[wasm_bindgen(constructor, catch, js_class = MediaStreamTrackProcessor)]
    pub(super) fn new(init: &JsValue) -> Result<MediaStreamTrackProcessor, JsValue>;

    #[wasm_bindgen(method, getter)]
    pub(super) fn readable(this: &MediaStreamTrackProcessor) -> JsValue;
}
