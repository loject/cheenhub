//! Маршрут политики обработки персональных данных.

use dioxus::prelude::*;

use crate::features::legal::{LegalDocumentPage, privacy_policy};

/// Показывает актуальную политику обработки персональных данных.
#[component]
pub(crate) fn PrivacyPolicy() -> Element {
    rsx! {
        LegalDocumentPage { document: privacy_policy() }
    }
}
