//! Доступный индикатор явного выключения микрофона участником.

use dioxus::prelude::*;

/// Показывает перечёркнутый микрофон рядом с именем или поверх аватара.
#[component]
pub(crate) fn MicrophoneMutedBadge(#[props(default)] compact: bool) -> Element {
    rsx! {
        span {
            class: if compact { "flex h-3 w-3 shrink-0 items-center justify-center rounded-full bg-zinc-950 text-red-300 ring-1 ring-zinc-950" } else { "inline-flex shrink-0 items-center text-red-300" },
            role: "img",
            "aria-label": "Микрофон выключен",
            title: "Микрофон выключен",
            svg {
                class: if compact { "h-2.5 w-2.5" } else { "h-3.5 w-3.5" },
                fill: "none", stroke: "currentColor", stroke_width: "1.8",
                view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", stroke_linejoin: "round",
                    d: "M3 3l18 18M9 9v3a3 3 0 0 0 5.12 2.12M9 5a3 3 0 0 1 6 0v6M5 10v2a7 7 0 0 0 12 4.9M19 10v2a7 7 0 0 1-.46 2.5M12 19v3m-4 0h8"
                }
            }
        }
    }
}
