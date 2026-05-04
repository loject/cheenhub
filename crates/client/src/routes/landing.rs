//! Landing route component.

use dioxus::prelude::*;

use crate::features::landing::LandingPage;

#[component]
pub(crate) fn Landing() -> Element {
    rsx! {
        LandingPage {}
    }
}
