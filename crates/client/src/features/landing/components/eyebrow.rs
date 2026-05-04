//! Section eyebrow component.

use dioxus::prelude::*;

#[component]
pub(crate) fn Eyebrow(label: &'static str, dark: bool) -> Element {
    let class_name = if dark {
        "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-zinc-500"
    } else {
        "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-zinc-500"
    };

    rsx! {
        div { class: "{class_name}",
            span { class: "h-1.5 w-1.5 rounded-full bg-zinc-600" }
            "{label}"
        }
    }
}
