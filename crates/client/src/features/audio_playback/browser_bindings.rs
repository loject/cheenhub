//! Browser WebCodecs bindings used by audio playback.

use js_sys::{Float32Array, Promise};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = AudioDecoder)]
    #[derive(Clone)]
    pub(super) type AudioDecoder;

    #[wasm_bindgen(constructor, catch, js_class = AudioDecoder)]
    pub(super) fn new(init: &JsValue) -> Result<AudioDecoder, JsValue>;

    #[wasm_bindgen(static_method_of = AudioDecoder, js_name = isConfigSupported)]
    #[allow(dead_code)]
    pub(super) fn is_config_supported(config: &JsValue) -> Promise;

    #[wasm_bindgen(method, catch, js_name = configure)]
    pub(super) fn configure(this: &AudioDecoder, config: &JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = decode)]
    pub(super) fn decode(this: &AudioDecoder, chunk: &EncodedAudioChunk) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = close)]
    pub(super) fn close(this: &AudioDecoder) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = EncodedAudioChunk)]
    pub(super) type EncodedAudioChunk;

    #[wasm_bindgen(constructor, catch, js_class = EncodedAudioChunk)]
    pub(super) fn new(init: &JsValue) -> Result<EncodedAudioChunk, JsValue>;

    #[wasm_bindgen(js_name = AudioData)]
    pub(super) type AudioData;

    #[wasm_bindgen(method, getter, js_name = numberOfFrames)]
    pub(super) fn number_of_frames(this: &AudioData) -> u32;

    #[wasm_bindgen(method, getter, js_name = numberOfChannels)]
    pub(super) fn number_of_channels(this: &AudioData) -> u32;

    #[wasm_bindgen(method, getter, js_name = sampleRate)]
    pub(super) fn sample_rate(this: &AudioData) -> f64;

    #[wasm_bindgen(method, catch, js_name = copyTo)]
    pub(super) fn copy_to(
        this: &AudioData,
        destination: &Float32Array,
        options: &JsValue,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = close)]
    pub(super) fn close(this: &AudioData) -> Result<(), JsValue>;
}
