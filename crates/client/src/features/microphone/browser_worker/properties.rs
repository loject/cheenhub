//! Чтение диагностических полей сообщений browser worker.

use js_sys::Reflect;
use wasm_bindgen::JsValue;

/// Читает строковое поле сообщения.
pub(super) fn string_property(value: &JsValue, name: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
}

/// Читает числовое поле сообщения.
pub(super) fn number_property(value: &JsValue, name: &str) -> Option<f64> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_f64())
}

/// Читает логическое поле сообщения.
pub(super) fn bool_property(value: &JsValue, name: &str) -> Option<bool> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_bool())
}
