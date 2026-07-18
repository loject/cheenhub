//! Browser-реализация проверки фокуса приложения.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

/// Возвращает, видима ли вкладка и принадлежит ли ей фокус.
pub(crate) fn application_is_focused() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| !document.hidden() && document.has_focus().unwrap_or(false))
}
