//! Маркер маршрута личного диалога.

use dioxus::prelude::*;

/// Рендерит пустой дочерний маршрут, пока layout приложения читает URL.
#[component]
pub(crate) fn AppDirectMessage(conversation_id: String) -> Element {
    let _ = conversation_id;
    rsx! {}
}
