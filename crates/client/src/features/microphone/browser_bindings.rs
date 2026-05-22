//! Browser WebCodecs and stream bindings for microphone capture.

use js_sys::{Float32Array, Promise, Uint8Array};
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

    #[wasm_bindgen(method, getter, js_name = numberOfFrames)]
    pub(super) fn audio_data_number_of_frames(this: &AudioData) -> u32;

    #[wasm_bindgen(method, getter, js_name = numberOfChannels)]
    pub(super) fn audio_data_number_of_channels(this: &AudioData) -> u32;

    #[wasm_bindgen(method, getter, js_name = sampleRate)]
    pub(super) fn audio_data_sample_rate(this: &AudioData) -> f64;

    #[wasm_bindgen(method, getter, js_name = timestamp)]
    pub(super) fn audio_data_timestamp(this: &AudioData) -> f64;

    #[wasm_bindgen(method, getter, js_name = duration)]
    pub(super) fn audio_data_duration(this: &AudioData) -> Option<f64>;

    #[wasm_bindgen(method, catch, js_name = copyTo)]
    pub(super) fn audio_data_copy_to(
        this: &AudioData,
        destination: &Float32Array,
        options: &JsValue,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(js_name = MediaStreamTrackProcessor)]
    #[derive(Clone)]
    pub(super) type MediaStreamTrackProcessor;

    #[wasm_bindgen(constructor, catch, js_class = MediaStreamTrackProcessor)]
    pub(super) fn new(init: &JsValue) -> Result<MediaStreamTrackProcessor, JsValue>;

    #[wasm_bindgen(method, getter)]
    pub(super) fn readable(this: &MediaStreamTrackProcessor) -> JsValue;
}
