//! Registration route component.

use dioxus::prelude::*;

use crate::features::auth::RegisterPage;

#[component]
pub(crate) fn Register() -> Element {
    rsx! {
        RegisterPage {}
    }
}
