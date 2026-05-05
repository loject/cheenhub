//! Authenticated application route component.

use dioxus::prelude::*;

use crate::features::app::AppPage;

#[component]
pub(crate) fn AppHome() -> Element {
    rsx! {
        AppPage {}
    }
}
