//! Section intro component.

use dioxus::prelude::*;

use crate::features::landing::components::eyebrow::Eyebrow;

#[component]
pub(crate) fn SectionIntro(
    eyebrow: &'static str,
    title: &'static str,
    description: &'static str,
) -> Element {
    rsx! {
        div { class: "mb-7",
            Eyebrow { label: eyebrow, dark: false }
            h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "{title}" }
            p { class: "mt-2 text-[14px] text-zinc-500", "{description}" }
        }
    }
}
