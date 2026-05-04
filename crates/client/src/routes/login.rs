//! Login route component.

use dioxus::prelude::*;

use crate::features::auth::LoginPage;

#[component]
pub(crate) fn Login() -> Element {
    rsx! {
        LoginPage {}
    }
}
