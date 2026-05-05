//! Reusable modal component for authenticated app flows.

use dioxus::prelude::*;

/// Renders a centered modal with a backdrop.
#[component]
pub(crate) fn Modal(title: &'static str, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-[100] flex items-center justify-center bg-black/65 px-4 py-6 backdrop-blur-sm",
            button {
                r#type: "button",
                class: "absolute inset-0 cursor-default",
                "aria-label": "Закрыть окно",
                onclick: move |_| on_close.call(()),
            }
            section {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": title,
                class: "relative w-full max-w-[420px] rounded-2xl border border-zinc-800 bg-zinc-950 p-5 text-zinc-100 shadow-[0_28px_90px_rgba(0,0,0,0.55)]",
                div { class: "mb-5 flex items-center justify-between gap-4",
                    h2 { class: "text-lg font-semibold text-zinc-50", "{title}" }
                    button {
                        r#type: "button",
                        class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100",
                        "aria-label": "Закрыть окно",
                        onclick: move |_| on_close.call(()),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 6l12 12M18 6 6 18" }
                        }
                    }
                }
                {children}
            }
        }
    }
}
