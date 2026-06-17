//! Компонент кнопки серверной панели.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use super::server_avatar::ServerAvatar;

/// Рендерит кнопку переключения серверов.
#[component]
pub(crate) fn ServerRailButton(
    server: ServerSummary,
    active: bool,
    on_select_server: EventHandler<String>,
) -> Element {
    let button_class = if active {
        "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px relative flex h-12 w-12 items-center justify-center rounded-2xl border border-accent/40 bg-accent text-sm font-semibold text-white"
    } else {
        "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-sm font-semibold text-zinc-300 hover:border-zinc-700 hover:bg-zinc-800"
    };
    let server_id = server.id.clone();

    rsx! {
        button {
            r#type: "button",
            class: button_class,
            "aria-label": "{server.name}",
            onclick: move |_| on_select_server.call(server_id.clone()),
            if active {
                span { class: "absolute -left-3 h-7 w-1 rounded-r-full bg-accent" }
            }
            ServerAvatar {
                name: server.name.clone(),
                avatar_url: server.avatar_url.clone(),
                class: "flex h-full w-full items-center justify-center overflow-hidden rounded-2xl text-sm font-semibold".to_owned(),
            }
        }
    }
}
