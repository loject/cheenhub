//! Маршрут пользовательского соглашения.

use dioxus::prelude::*;

use crate::features::legal::{LegalDocumentPage, terms};

/// Показывает актуальное пользовательское соглашение.
#[component]
pub(crate) fn Terms(return_to: Option<String>) -> Element {
    rsx! {
        LegalDocumentPage { document: terms(), return_to_registration: return_to.as_deref() == Some("registration") }
    }
}
