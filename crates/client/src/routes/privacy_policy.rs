//! Маршрут политики обработки персональных данных.

use dioxus::prelude::*;

use crate::features::legal::{LegalDocumentPage, privacy_policy};

/// Показывает актуальную политику обработки персональных данных.
#[component]
pub(crate) fn PrivacyPolicy(return_to: Option<String>) -> Element {
    rsx! {
        LegalDocumentPage { document: privacy_policy(), return_to_registration: return_to.as_deref() == Some("registration") }
    }
}
