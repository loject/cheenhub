//! Маркер маршрута сервера без выбранной комнаты.

use dioxus::prelude::*;

/// Рендерит пустой дочерний маршрут, пока layout приложения читает URL.
#[component]
pub(crate) fn AppServer(server_id: String) -> Element {
    let _ = server_id;
    rsx! {}
}
