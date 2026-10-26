//! Доступный индикатор отключённого входящего звука участника.

use dioxus::prelude::*;

/// Показывает перечёркнутый динамик рядом с именем или поверх аватара.
#[component]
pub(crate) fn OutputMutedBadge(#[props(default)] compact: bool) -> Element {
    rsx! {
        span {
            class: if compact { "flex h-3 w-3 shrink-0 items-center justify-center rounded-full bg-zinc-950 text-sky-300 ring-1 ring-zinc-950" } else { "inline-flex shrink-0 items-center text-sky-300" },
            role: "img",
            "aria-label": "Входящий звук выключен",
            title: "Не слышит других участников",
            svg {
                class: if compact { "h-2.5 w-2.5" } else { "h-3.5 w-3.5" },
                fill: "none", stroke: "currentColor", stroke_width: "1.8",
                view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", stroke_linejoin: "round",
                    d: "M11 5 6 9H3v6h3l5 4V5Z"
                }
                path { stroke_linecap: "round", stroke_linejoin: "round",
                    d: "M16 9l5 6m0-6-5 6"
                }
            }
        }
    }
}
