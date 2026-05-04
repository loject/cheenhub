//! Landing page features section component.

use dioxus::prelude::*;

use crate::features::landing::components::feature_card::FeatureCard;
use crate::features::landing::components::section_intro::SectionIntro;
use crate::features::landing::data::FEATURES;

#[component]
pub(crate) fn FeaturesSection() -> Element {
    rsx! {
        section { id: "features", class: "mx-auto max-w-6xl px-5 pb-20 lg:px-8",
            SectionIntro {
                eyebrow: "Возможности",
                title: "Сделано правильно.",
                description: "Не очередной клон — конкретные решения конкретных проблем."
            }
            div { class: "grid gap-3 sm:grid-cols-2 lg:grid-cols-3",
                for feature in FEATURES {
                    FeatureCard { feature: *feature }
                }
            }
        }
    }
}
