//! Feature card component.

use dioxus::prelude::*;

use crate::features::landing::components::feature_svg::FeatureSvg;
use crate::features::landing::data::Feature;

#[component]
pub(crate) fn FeatureCard(feature: Feature) -> Element {
    rsx! {
        div { class: "fcard rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-5",
            div { class: "mb-3 flex h-9 w-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-300",
                FeatureSvg { icon: feature.icon }
            }
            div { class: "text-[13px] font-semibold text-zinc-100", "{feature.title}" }
            div { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "{feature.description}" }
        }
    }
}
