//! Компонент маршрута принятия приглашения.

use dioxus::prelude::*;

use crate::features::app::InvitePage;

#[component]
pub(crate) fn Invite(code: String) -> Element {
    rsx! {
        InvitePage { code }
    }
}
