//! Маркер маршрута почтовых настроек хоста.

use dioxus::prelude::*;

/// Рендерит пустой дочерний маршрут, пока layout приложения строит workspace.
#[component]
pub(crate) fn AppHostEmailSettings(gmail: Option<String>, email: Option<String>) -> Element {
    let _ = (gmail, email);
    rsx! {}
}
