//! Проверка и фиксация юридических подтверждений при регистрации.

use cheenhub_contracts::legal::{
    PERSONAL_DATA_CONSENT_VERSION, PRIVACY_POLICY_VERSION, TERMS_VERSION,
};
use uuid::Uuid;

use crate::features::auth::domain::RegistrationLegalAcceptance;
use crate::features::auth::error::AuthError;

/// Проверяет два независимых подтверждения регистрационной формы.
pub(super) fn validate_registration_acceptance(
    accepts_terms: bool,
    accepts_personal_data: bool,
    registration_kind: &'static str,
) -> Result<(), AuthError> {
    if !accepts_terms {
        tracing::warn!(
            registration_kind,
            "rejected registration without terms acceptance"
        );
        return Err(AuthError::BadRequest(
            "Нужно принять пользовательское соглашение.".to_owned(),
        ));
    }
    if !accepts_personal_data {
        tracing::warn!(
            registration_kind,
            "rejected registration without personal data consent"
        );
        return Err(AuthError::BadRequest(
            "Нужно отдельно дать согласие на обработку персональных данных.".to_owned(),
        ));
    }

    Ok(())
}

/// Возвращает версии документов, действующие для текущей регистрации.
pub(super) fn current_acceptance(registration_kind: &'static str) -> RegistrationLegalAcceptance {
    RegistrationLegalAcceptance {
        acceptance_source: registration_kind.to_owned(),
        terms_version: TERMS_VERSION.to_owned(),
        privacy_policy_version: PRIVACY_POLICY_VERSION.to_owned(),
        personal_data_consent_version: PERSONAL_DATA_CONSENT_VERSION.to_owned(),
    }
}

/// Записывает безопасное диагностическое событие фиксации подтверждений.
pub(super) fn log_recorded(user_id: &Uuid, registration_kind: &'static str) {
    tracing::info!(
        %user_id,
        registration_kind,
        terms_version = TERMS_VERSION,
        privacy_policy_version = PRIVACY_POLICY_VERSION,
        personal_data_consent_version = PERSONAL_DATA_CONSENT_VERSION,
        "recorded registration legal acceptances"
    );
}
