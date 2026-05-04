//! Landing route page.

use dioxus::prelude::*;

use crate::features::landing::components::comparison_section::ComparisonSection;
use crate::features::landing::components::cta_section::CtaSection;
use crate::features::landing::components::features_section::FeaturesSection;
use crate::features::landing::components::footer::Footer;
use crate::features::landing::components::header::Header;
use crate::features::landing::components::hero::Hero;
use crate::features::landing::components::tech_section::TechSection;

#[component]
pub(crate) fn LandingPage() -> Element {
    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            div { class: "grid-bg min-h-screen",
                Header {}
                Hero {}
                FeaturesSection {}
                ComparisonSection {}
                TechSection {}
                CtaSection {}
                Footer {}
            }
        }
    }
}
