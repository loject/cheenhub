//! Минимальный wasm API будущего worker микрофона.

use wasm_bindgen::prelude::*;

const MICROPHONE_WORKER_ABI_VERSION: u32 = 1;

/// Возвращает версию ABI worker микрофона для проверки загрузки wasm-модуля.
#[wasm_bindgen]
pub fn microphone_worker_abi_version() -> u32 {
    MICROPHONE_WORKER_ABI_VERSION
}
