//! Заглушки списка экранов на время загрузки превью.

use dioxus::prelude::*;

/// Показывает сетку карточек-заглушек, пока грузятся превью экранов.
#[component]
pub(super) fn ListSkeleton() -> Element {
    rsx! {
        div {
            class: "space-y-2",
            role: "status",
            "aria-live": "polite",
            "aria-label": "Загружаем доступные экраны",
            div { class: "grid grid-cols-2 gap-2",
                for index in 0..4 {
                    div {
                        key: "{index}",
                        class: "overflow-hidden rounded-lg border border-zinc-800 bg-zinc-900/40",
                        div { class: "aspect-video animate-pulse bg-zinc-800/70" }
                        div { class: "space-y-1.5 px-2 py-1.5",
                            div { class: "h-2.5 w-2/3 animate-pulse rounded bg-zinc-800" }
                            div { class: "h-2 w-1/3 animate-pulse rounded bg-zinc-800/70" }
                        }
                    }
                }
            }
        }
    }
}
