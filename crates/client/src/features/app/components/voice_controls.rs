//! Floating voice controls component.

use dioxus::prelude::*;

/// Renders the static floating controls for voice rooms.
#[component]
pub(crate) fn VoiceControls() -> Element {
    rsx! {
        div { class: "voice-floating-controls pointer-events-none absolute inset-x-0 bottom-0 flex justify-center p-5",
            div { class: "pointer-events-auto flex items-center gap-3 rounded-[24px] border border-zinc-800 bg-zinc-950/85 p-2 shadow-[0_18px_50px_rgba(0,0,0,0.38)] drop-shadow-[0_12px_24px_rgba(0,0,0,.28)] backdrop-blur-xl",
                button { r#type: "button", class: "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Микрофон",
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Микрофон" }
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                    }
                }
                button { r#type: "button", class: "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Экран",
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Экран" }
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                        path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
                    }
                }
                button { r#type: "button", class: "group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-300 transition-[transform,background,border-color,color,box-shadow,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-red-500/30 hover:bg-red-500/10 hover:text-red-200", "aria-label": "Выйти",
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Выйти" }
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                    }
                }
            }
        }
    }
}
