//! Вспомогательные функции ошибок браузерной демонстрации экрана.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use js_sys::Reflect;
use wasm_bindgen::JsValue;

pub(super) fn is_permission_denied_error(error: &JsValue) -> bool {
    let name = Reflect::get(error, &JsValue::from_str("name"))
        .ok()
        .and_then(|value| value.as_string())
        .unwrap_or_default();
    matches!(
        name.as_str(),
        "NotAllowedError" | "PermissionDeniedError" | "SecurityError"
    )
}

pub(super) fn js_error_message(error: JsValue) -> String {
    if let Some(message) = Reflect::get(&error, &JsValue::from_str("message"))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
    {
        return message;
    }

    if let Some(value) = error.as_string().filter(|value| !value.is_empty()) {
        return value;
    }

    "Неизвестная ошибка браузера.".to_string()
}
