//! Web-реализация маршрута лендинга.
#![cfg_attr(not(target_family = "wasm"), allow(dead_code, unused_imports))]

use dioxus::prelude::*;

use super::pages::landing_page::LandingPage;
use crate::Route;

/// Рендерит web-лендинг.
#[component]
pub(crate) fn LandingRoute() -> Element {
    rsx! {
        LandingPage {}
    }
}

/// Возвращает домашний маршрут web-клиента.
pub(crate) fn public_home_route() -> Route {
    Route::Landing {}
}

/// Возвращает подпись домашнего действия web-клиента.
pub(crate) fn public_home_label() -> &'static str {
    "На главную"
}

/// Возвращает доступность публичного лендинга на web-клиенте.
pub(crate) fn public_landing_available() -> bool {
    true
}
