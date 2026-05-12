//! Reset password route component.

use dioxus::prelude::*;

use crate::features::auth::ResetPasswordPage;

#[component]
pub(crate) fn ResetPassword(token: Option<String>) -> Element {
    rsx! {
        ResetPasswordPage { token }
    }
}
