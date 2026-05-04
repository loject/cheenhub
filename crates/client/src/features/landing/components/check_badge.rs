//! Check badge component.

use dioxus::prelude::*;

use crate::features::landing::components::check_icon::CheckIcon;

#[component]
pub(crate) fn CheckBadge(label: &'static str) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-1.5 rounded-full border border-zinc-700 bg-zinc-900 px-2.5 py-1 text-[11px] text-zinc-300",
            CheckIcon { class_name: "h-3 w-3" }
            "{label}"
        }
    }
}
