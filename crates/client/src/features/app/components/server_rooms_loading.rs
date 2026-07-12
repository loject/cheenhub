//! Индикатор загрузки списка комнат сервера.

use dioxus::prelude::*;

/// Показывает скелетон списка комнат во время первоначальной загрузки.
#[component]
pub(super) fn ServerRoomsLoading() -> Element {
    rsx! {
        div { class: "space-y-2 px-1 py-2",
            div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/80" }
            div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/60" }
            div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/40" }
        }
    }
}
