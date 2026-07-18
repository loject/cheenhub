//! Stub-реализация для неподдерживаемых платформ.
#![allow(dead_code, unused_imports)]

use dioxus::prelude::*;

/// На неподдерживаемых платформах уведомления не работают.
#[component]
pub(crate) fn NotificationsProvider(children: Element) -> Element {
    rsx! {
        {children}
    }
}
