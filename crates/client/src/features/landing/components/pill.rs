//! Compact landing page pill component.

use dioxus::prelude::*;

#[component]
pub(crate) fn Pill(strong: &'static str, text: &'static str) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] text-zinc-400",
            span { class: "font-medium text-zinc-200", "{strong}" }
            "{text}"
        }
    }
}
