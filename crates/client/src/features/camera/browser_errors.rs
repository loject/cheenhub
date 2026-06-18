//! Вспомогательные функции ошибок браузерной камеры.

use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

pub(super) fn js_error_message(error: JsValue) -> String {
    error
        .dyn_ref::<js_sys::Error>()
        .map(js_sys::Error::message)
        .and_then(|message| message.as_string())
        .filter(|message| !message.is_empty())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "Неизвестная ошибка браузера.".to_owned())
}

pub(super) fn is_permission_denied_error(error: &JsValue) -> bool {
    let name_denied = error
        .dyn_ref::<web_sys::DomException>()
        .map(web_sys::DomException::name)
        .or_else(|| {
            Reflect::get(error, &JsValue::from_str("name"))
                .ok()
                .and_then(|name| name.as_string())
        })
        .is_some_and(|name| {
            name == "NotAllowedError" || name == "PermissionDeniedError" || name == "SecurityError"
        });
    if name_denied {
        return true;
    }

    let message = js_error_message(error.clone()).to_ascii_lowercase();
    message.contains("permission denied")
        || message.contains("permission dismissed")
        || message.contains("permission denied by system")
        || message.contains("notallowederror")
        || message.contains("permissiondeniederror")
        || message.contains("denied permission")
        || message.contains("access to the device is not allowed")
}
