//! Карточка экрана в сетке выбора источника.

use dioxus::prelude::*;

use super::super::model::ScreenShareSource;

/// Показывает превью экрана и сообщает его идентификатор при выборе.
#[component]
pub(super) fn SourceCard(
    source: ScreenShareSource,
    selected: bool,
    on_select: EventHandler<String>,
) -> Element {
    let class = if selected {
        "group relative overflow-hidden rounded-lg border border-blue-400/60 bg-blue-500/[.08] text-left transition-[border-color,background-color] duration-200 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300"
    } else {
        "group relative overflow-hidden rounded-lg border border-zinc-800 bg-zinc-900/40 text-left transition-[border-color,background-color] duration-200 hover:border-zinc-700 hover:bg-zinc-900 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300"
    };
    let source_id = source.id.clone();

    rsx! {
        button {
            r#type: "button",
            role: "radio",
            "aria-checked": if selected { "true" } else { "false" },
            "aria-label": "{source.name}, {source.width} на {source.height} пикселей",
            class,
            onclick: move |_| on_select.call(source_id.clone()),
            div { class: "relative aspect-video overflow-hidden bg-black",
                img {
                    class: "h-full w-full object-contain",
                    src: "{source.preview_url}",
                    alt: "Предварительный просмотр: {source.name}",
                }
                if source.is_primary {
                    span { class: "absolute left-1.5 top-1.5 rounded border border-zinc-700 bg-black/70 px-1.5 py-0.5 text-[9.5px] font-semibold text-zinc-200", "Основной" }
                }
                if selected {
                    span { class: "screen-share-pop absolute right-1.5 top-1.5 grid h-5 w-5 place-items-center rounded-full bg-blue-500 text-white", "aria-hidden": "true",
                        svg { class: "h-3 w-3", fill: "none", stroke: "currentColor", stroke_width: "2.6", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m5 12 4 4L19 6" }
                        }
                    }
                }
            }
            div { class: "flex items-center justify-between gap-2 px-2 py-1.5",
                span { class: "truncate text-[11.5px] font-medium text-zinc-100", "{source.name}" }
                span { class: "shrink-0 text-[10px] tabular-nums text-zinc-500", "{source.width}×{source.height}" }
            }
        }
    }
}
