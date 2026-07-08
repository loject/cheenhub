//! Заглушка скачивания native-клиента вне web-клиента.

use dioxus::prelude::*;

/// Не рендерит web-only блок скачивания native-клиента.
#[component]
pub(crate) fn NativeClientDownload() -> Element {
    rsx! {}
}
