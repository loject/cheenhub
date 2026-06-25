//! Browser WebCodecs bindings for microphone capture.

use js_sys::{Promise, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = AudioEncoder)]
    #[derive(Clone)]
    pub(super) type AudioEncoder;

    #[wasm_bindgen(constructor, catch, js_class = AudioEncoder)]
    pub(super) fn new(init: &JsValue) -> Result<AudioEncoder, JsValue>;

    #[wasm_bindgen(static_method_of = AudioEncoder, js_name = isConfigSupported)]
    pub(super) fn is_config_supported(config: &JsValue) -> Promise;

    #[wasm_bindgen(method, catch, js_name = configure)]
    pub(super) fn configure(this: &AudioEncoder, config: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = encode)]
    pub(super) fn encode(this: &AudioEncoder, data: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(method, getter, js_name = encodeQueueSize)]
    pub(super) fn encode_queue_size(this: &AudioEncoder) -> u32;

    #[wasm_bindgen(method, catch, js_name = close)]
    pub(super) fn close(this: &AudioEncoder) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = EncodedAudioChunk)]
    #[derive(Clone)]
    pub(super) type EncodedAudioChunk;

    #[wasm_bindgen(method, getter, js_name = byteLength)]
    pub(super) fn byte_length(this: &EncodedAudioChunk) -> u32;

    #[wasm_bindgen(method, getter)]
    pub(super) fn timestamp(this: &EncodedAudioChunk) -> f64;

    #[wasm_bindgen(method, getter)]
    pub(super) fn duration(this: &EncodedAudioChunk) -> Option<f64>;

    #[wasm_bindgen(method, catch, js_name = copyTo)]
    pub(super) fn copy_to(
        this: &EncodedAudioChunk,
        destination: &Uint8Array,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = AudioData)]
    #[derive(Clone)]
    pub(super) type AudioData;

    #[wasm_bindgen(constructor, catch, js_class = AudioData)]
    pub(super) fn new(init: &JsValue) -> Result<AudioData, JsValue>;

    #[wasm_bindgen(method, catch, js_name = close)]
    pub(super) fn close(this: &AudioData) -> Result<(), JsValue>;
}
