//! Выбор локальной пасхалки либо обычного сохранения данных профиля.

use crate::features::developer_mode::is_activation_pair;
use cheenhub_contracts::rest::UpdateCurrentUserRequest;

/// Результат проверки формы до любого обращения к API профиля.
pub(super) enum ProfileSubmission {
    /// Включить локальный режим, не изменяя профиль на сервере.
    ActivateDeveloperMode,
    /// Передать API только поддерживаемое изменение никнейма.
    UpdateNickname(UpdateCurrentUserRequest),
    /// Отклонить попытку смены email, которую API пока не поддерживает.
    RejectEmailChange,
}

/// Выбирает сохранение после проверки валидности никнейма вызывающим компонентом.
///
/// Пасхалка имеет приоритет перед запретом смены email; сравнение ее значений точное.
pub(super) fn profile_submission(
    nickname: &str,
    email: &str,
    current_email: &str,
) -> ProfileSubmission {
    if is_activation_pair(nickname, email) {
        ProfileSubmission::ActivateDeveloperMode
    } else if email != current_email {
        ProfileSubmission::RejectEmailChange
    } else {
        ProfileSubmission::UpdateNickname(UpdateCurrentUserRequest {
            nickname: nickname.trim().to_owned(),
        })
    }
}

#[cfg(test)]
mod tests;
