//! Компактный значок активного голосового подключения для панели серверов.

use dioxus::prelude::*;

/// Показывает на иконке сервера или CheenHub подтверждённое подключение к голосу.
#[component]
pub(super) fn VoiceConnectedBadge() -> Element {
    rsx! {
        span {
            class: "pointer-events-none absolute -bottom-1 -right-1 z-10 flex size-5 items-center justify-center rounded-full border-2 border-zinc-950 bg-emerald-400 text-zinc-950 shadow-[0_2px_8px_rgba(16,185,129,0.35)]",
            "aria-hidden": "true",
            svg {
                class: "size-3",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                view_box: "0 0 24 24",
                path { d: "M4 14v-3a8 8 0 0 1 16 0v3" }
                path { d: "M4 13h2a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-3a2 2 0 0 1 1-2Z" }
                path { d: "M20 13h-2a2 2 0 0 0-2 2v3a2 2 0 0 0 2 2h1a2 2 0 0 0 2-2v-3a2 2 0 0 0-1-2Z" }
            }
        }
    }
}
