//! Рабочая область аутентифицированного приложения для одного сервера.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use super::app_shell::{AppModal, ServerShellState};
use super::server_rooms_scope::ServerRoomsScope;

/// Рендерит одну обертку рабочей области сервера.
#[component]
pub(crate) fn ServerInstance(
    server: ServerSummary,
    active: bool,
    on_state_change: EventHandler<(String, ServerShellState)>,
    on_open_modal: EventHandler<AppModal>,
    on_left_server: EventHandler<String>,
    on_server_updated: EventHandler<ServerSummary>,
) -> Element {
    let wrapper_class = if active { "contents" } else { "hidden" };

    rsx! {
        div { class: wrapper_class,
            ServerRoomsScope {
                server,
                active,
                on_state_change,
                on_open_modal,
                on_left_server,
                on_server_updated,
            }
        }
    }
}
