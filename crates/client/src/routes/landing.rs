//! Компонент маршрута лендинга.

use dioxus::prelude::*;

use crate::features::landing::LandingRoute;

#[component]
pub(crate) fn Landing() -> Element {
    rsx! {
        LandingRoute {}
    }
}
