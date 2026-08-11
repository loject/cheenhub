//! Маршрут согласия на обработку персональных данных.

use dioxus::prelude::*;

use crate::features::legal::{LegalDocumentPage, personal_data_consent};

/// Показывает актуальное согласие на обработку персональных данных.
#[component]
pub(crate) fn PersonalDataConsent() -> Element {
    rsx! {
        LegalDocumentPage { document: personal_data_consent() }
    }
}
