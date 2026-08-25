//! Маркер основного маршрута настроек хоста.

use dioxus::prelude::*;

/// Рендерит пустой дочерний маршрут, пока layout приложения строит workspace.
#[component]
pub(crate) fn AppHostSettings() -> Element {
    rsx! {}
}
