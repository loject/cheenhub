//! Переключатель между экранами и окнами приложений.

use dioxus::prelude::*;

use super::super::model::ScreenShareSourceTab;
use super::decor::aria_flag;

/// Переключает раздел выбора источника с плавным сдвигом активной подложки.
#[component]
pub(super) fn KindTabs(
    active: ScreenShareSourceTab,
    on_change: EventHandler<ScreenShareSourceTab>,
) -> Element {
    let screens = active == ScreenShareSourceTab::Screens;
    let indicator = if screens {
        "translate-x-0"
    } else {
        "translate-x-full"
    };

    rsx! {
        div {
            class: "relative grid grid-cols-2 rounded-lg bg-zinc-900/60 p-1 ring-1 ring-zinc-800",
            role: "tablist",
            "aria-label": "Тип источника демонстрации",
            div { class: "pointer-events-none absolute inset-y-1 left-1 w-[calc(50%-4px)] rounded-md bg-zinc-800 transition-transform duration-300 ease-out {indicator}" }
            button {
                r#type: "button",
                role: "tab",
                "aria-selected": aria_flag(screens),
                class: if screens {
                    "relative z-10 flex min-h-8 items-center justify-center gap-1.5 rounded-md px-2 text-[12px] font-medium text-white transition-colors duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
                } else {
                    "relative z-10 flex min-h-8 items-center justify-center gap-1.5 rounded-md px-2 text-[12px] font-medium text-zinc-400 transition-colors duration-200 hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
                },
                onclick: move |_| on_change.call(ScreenShareSourceTab::Screens),
                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                    rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                    path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
                }
                "Весь экран"
            }
            button {
                r#type: "button",
                role: "tab",
                "aria-selected": aria_flag(!screens),
                class: if screens {
                    "relative z-10 flex min-h-8 items-center justify-center gap-1.5 rounded-md px-2 text-[12px] font-medium text-zinc-400 transition-colors duration-200 hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
                } else {
                    "relative z-10 flex min-h-8 items-center justify-center gap-1.5 rounded-md px-2 text-[12px] font-medium text-white transition-colors duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70"
                },
                onclick: move |_| on_change.call(ScreenShareSourceTab::Windows),
                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                    rect { x: "3", y: "5", width: "18", height: "14", rx: "2" }
                    path { stroke_linecap: "round", d: "M3 9h18M7 7h.01M10 7h.01" }
                }
                "Окно приложения"
            }
        }
    }
}
