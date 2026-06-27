//! Запасная PWA-интеграция для платформ без service worker.

use dioxus::prelude::*;

/// Не выполняет PWA-регистрацию на платформах без service worker.
#[component]
pub(crate) fn PwaVersionBridge() -> Element {
    rsx! {}
}
