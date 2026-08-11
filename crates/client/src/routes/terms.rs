//! Маршрут пользовательского соглашения.

use dioxus::prelude::*;

use crate::features::legal::{LegalDocumentPage, terms};

/// Показывает актуальное пользовательское соглашение.
#[component]
pub(crate) fn Terms() -> Element {
    rsx! {
        LegalDocumentPage { document: terms() }
    }
}
