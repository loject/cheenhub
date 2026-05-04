//! Authentication option metric component.

use dioxus::prelude::*;

#[component]
pub(crate) fn AuthMetric(value: &'static str, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/80 p-4",
            div { class: "text-[13px] font-semibold text-zinc-100", "{value}" }
            div { class: "mt-1 text-[11px] text-zinc-600", "{label}" }
        }
    }
}
