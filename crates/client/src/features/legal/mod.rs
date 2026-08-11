//! Публичные юридические документы CheenHub.

mod document_page;
mod personal_data_consent;
mod privacy_policy;
mod terms;

pub(crate) use document_page::LegalDocumentPage;
pub(crate) use personal_data_consent::personal_data_consent;
pub(crate) use privacy_policy::privacy_policy;
pub(crate) use terms::terms;
