//! Маршрут согласия на обработку персональных данных.

use dioxus::prelude::*;

use crate::features::legal::{LegalDocumentPage, personal_data_consent};

/// Показывает актуальное согласие на обработку персональных данных.
#[component]
pub(crate) fn PersonalDataConsent(return_to: Option<String>) -> Element {
    rsx! {
        LegalDocumentPage { document: personal_data_consent(), return_to_registration: return_to.as_deref() == Some("registration") }
    }
}
