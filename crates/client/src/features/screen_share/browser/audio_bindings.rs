//! WebCodecs API для Opus системного звука, изолированный от микрофонного DSP.

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Browser-кодировщик, выдающий Opus-чанки через output callback.
    #[wasm_bindgen(js_name = AudioEncoder)]
    #[derive(Clone)]
    pub(super) type AudioEncoder;
    /// Создаёт кодировщик с output/error callback из init.
    #[wasm_bindgen(constructor, catch, js_class = AudioEncoder)]
    pub(super) fn new(init: &JsValue) -> Result<AudioEncoder, JsValue>;
    /// Применяет согласованные codec, sampleRate, channels и bitrate.
    #[wasm_bindgen(method, catch)]
    pub(super) fn configure(this: &AudioEncoder, config: &JsValue) -> Result<(), JsValue>;
    /// Ставит один PCM-кадр в очередь WebCodecs.
    #[wasm_bindgen(method, catch)]
    pub(super) fn encode(this: &AudioEncoder, data: &AudioData) -> Result<(), JsValue>;
    /// Возвращает число ожидающих PCM-кадров для ограничения нагрузки.
    #[wasm_bindgen(method, getter, js_name = encodeQueueSize)]
    pub(super) fn queue_size(this: &AudioEncoder) -> u32;
    /// Закрывает кодировщик и отменяет его незавершённую работу.
    #[wasm_bindgen(method, catch)]
    pub(super) fn close(this: &AudioEncoder) -> Result<(), JsValue>;

    /// Владеющий PCM-кадр WebCodecs с явным освобождением после encode.
    #[wasm_bindgen(js_name = AudioData)]
    pub(super) type AudioData;
    /// Создаёт PCM-кадр из planar float samples и времени захвата.
    #[wasm_bindgen(constructor, catch, js_class = AudioData)]
    pub(super) fn new(init: &JsValue) -> Result<AudioData, JsValue>;
    /// Освобождает backing storage PCM-кадра.
    #[wasm_bindgen(method, catch)]
    pub(super) fn close(this: &AudioData) -> Result<(), JsValue>;

    /// Закодированный Opus-чанк, полученный через output callback.
    #[wasm_bindgen(js_name = EncodedAudioChunk)]
    pub(super) type EncodedAudioChunk;
    /// Возвращает размер Opus payload в байтах.
    #[wasm_bindgen(method, getter, js_name = byteLength)]
    pub(super) fn byte_length(this: &EncodedAudioChunk) -> u32;
    /// Возвращает timestamp первого sample в микросекундах.
    #[wasm_bindgen(method, getter)]
    pub(super) fn timestamp(this: &EncodedAudioChunk) -> f64;
    /// Возвращает длительность чанка в микросекундах, если браузер её указал.
    #[wasm_bindgen(method, getter)]
    pub(super) fn duration(this: &EncodedAudioChunk) -> Option<f64>;
    /// Копирует исходные Opus bytes в буфер нужного размера.
    #[wasm_bindgen(method, catch, js_name = copyTo)]
    pub(super) fn copy_to(this: &EncodedAudioChunk, target: &Uint8Array) -> Result<(), JsValue>;
}
