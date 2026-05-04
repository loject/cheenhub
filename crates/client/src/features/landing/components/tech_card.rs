//! Technology card component.

use dioxus::prelude::*;

use crate::features::landing::data::TechGroup;

#[component]
pub(crate) fn TechCard(group: TechGroup) -> Element {
    rsx! {
        div { class: "rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-4",
            div { class: "mb-3 text-[10px] uppercase tracking-[0.2em] text-zinc-600", "{group.title}" }
            div { class: "space-y-2",
                for item in group.items {
                    div { class: "flex items-center gap-2.5 rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2",
                        span { class: "w-6 text-center font-mono text-[12px] text-zinc-400", "{item.code}" }
                        span { class: "text-[12px] text-zinc-300", "{item.label}" }
                    }
                }
            }
        }
    }
}
