//! Компонент маршрута лендинга.

use dioxus::prelude::*;

use crate::features::landing::LandingPage;

#[component]
pub(crate) fn Landing() -> Element {
    rsx! {
        LandingPage {}
    }
}
