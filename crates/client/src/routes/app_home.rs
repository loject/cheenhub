//! Компонент маршрута аутентифицированного приложения.

use dioxus::prelude::*;

/// Рендерит пустой корневой app-маршрут, который layout заменит на workspace.
#[component]
pub(crate) fn AppHome() -> Element {
    rsx! {}
}
