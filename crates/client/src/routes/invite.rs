//! Invite acceptance route component.

use dioxus::prelude::*;

use crate::features::app::InvitePage;

#[component]
pub(crate) fn Invite(code: String) -> Element {
    rsx! {
        InvitePage { code }
    }
}
