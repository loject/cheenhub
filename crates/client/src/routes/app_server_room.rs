//! Маркер маршрута серверной комнаты.

use dioxus::prelude::*;

/// Рендерит пустой дочерний маршрут, пока layout приложения читает URL.
#[component]
pub(crate) fn AppServerRoom(server_id: String, room_id: String) -> Element {
    let _ = (server_id, room_id);
    rsx! {}
}
