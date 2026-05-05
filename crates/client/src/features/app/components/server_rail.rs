//! Server rail component for the authenticated app.

use dioxus::prelude::*;

use crate::features::landing::components::logo_icon::LogoIcon;

/// Renders the server switcher rail.
#[component]
pub(crate) fn ServerRail() -> Element {
    rsx! {
        aside { class: "flex w-[76px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/85 p-3 backdrop-blur-xl",
            div { class: "mb-4 flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-100 text-zinc-950",
                LogoIcon { class_name: "h-7 w-7" }
            }
            div { class: "space-y-2 overflow-y-auto pb-3",
                button { r#type: "button", class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px relative flex h-12 w-12 items-center justify-center rounded-2xl border border-accent/40 bg-accent text-sm font-semibold text-white", "aria-label": "CheenHub Dev",
                    span { class: "absolute -left-3 h-7 w-1 rounded-r-full bg-accent" }
                    "CH"
                }
                button { r#type: "button", class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-sm font-semibold text-zinc-300 hover:border-zinc-700 hover:bg-zinc-800", "aria-label": "Rust Gang", "RS" }
                button { r#type: "button", class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-sm font-semibold text-zinc-300 hover:border-zinc-700 hover:bg-zinc-800", "aria-label": "LAN Party", "LP" }
                button { r#type: "button", class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-sm font-semibold text-zinc-300 hover:border-zinc-700 hover:bg-zinc-800", "aria-label": "Design", "UI" }
            }
            div { class: "mt-auto border-t border-zinc-800/80 pt-3",
                button { r#type: "button", class: "transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 hover:border-accent/30 hover:bg-accent/10 hover:text-zinc-200", "aria-label": "Добавить сервер",
                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                    }
                }
            }
        }
    }
}
