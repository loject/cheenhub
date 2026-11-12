//! Преобразование ошибок хранилища личных звонков в ошибки приложения.

use crate::features::voice_chat::infrastructure::DirectCallStoreError;

use super::super::VoiceChatApplicationError;

pub(super) fn map_store_error(error: DirectCallStoreError) -> VoiceChatApplicationError {
    match error {
        DirectCallStoreError::CallerBusy => VoiceChatApplicationError::BadRequest(
            "У тебя уже есть незавершённый личный звонок.".to_owned(),
        ),
        DirectCallStoreError::NotFound => {
            VoiceChatApplicationError::NotFound("Звонок не найден.".to_owned())
        }
        DirectCallStoreError::Unauthorized => {
            VoiceChatApplicationError::Unauthorized("Нет доступа к звонку.".to_owned())
        }
        DirectCallStoreError::InvalidState => VoiceChatApplicationError::BadRequest(
            "Действие недоступно в текущем состоянии звонка.".to_owned(),
        ),
        DirectCallStoreError::Expired(_) => {
            VoiceChatApplicationError::NotFound("Время ожидания ответа истекло.".to_owned())
        }
    }
}

pub(super) fn store_error_kind(error: &DirectCallStoreError) -> &'static str {
    match error {
        DirectCallStoreError::CallerBusy => "caller_busy",
        DirectCallStoreError::NotFound => "not_found",
        DirectCallStoreError::Unauthorized => "unauthorized",
        DirectCallStoreError::InvalidState => "invalid_state",
        DirectCallStoreError::Expired(_) => "expired",
    }
}
