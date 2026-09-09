//! Состояние загрузки истории сообщений.

use dioxus::prelude::*;

/// Показывает спокойный скелетон входящего и исходящего сообщений.
#[component]
pub(crate) fn ChatHistoryLoadingState() -> Element {
    rsx! {
        div { class: "space-y-6 py-2", role: "status", "aria-live": "polite",
            div { class: "flex animate-pulse items-start gap-3.5",
                div { class: "h-9 w-9 shrink-0 rounded-full bg-zinc-800/90" }
                div { class: "space-y-2",
                    div { class: "h-2.5 w-24 rounded-full bg-zinc-800/80" }
                    div { class: "h-14 w-72 max-w-[65vw] rounded-[14px] bg-zinc-900" }
                }
            }
            div { class: "flex animate-pulse items-start justify-end gap-3.5",
                div { class: "space-y-2",
                    div { class: "ml-auto h-2.5 w-20 rounded-full bg-zinc-800/70" }
                    div { class: "h-12 w-64 max-w-[60vw] rounded-[14px] bg-blue-950/45" }
                }
                div { class: "h-9 w-9 shrink-0 rounded-full bg-zinc-800/80" }
            }
            span { class: "sr-only", "Загружаем сообщения" }
        }
    }
}
