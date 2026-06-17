//! Вспомогательные функции хранения пользователей для Postgres.

use crate::features::auth::infrastructure::{InsertUserError, UserConflict};

/// Сопоставляет ошибки вставки в базу с конфликтами полей пользователя.
pub(super) fn map_insert_user_error(error: sea_orm::DbErr) -> InsertUserError {
    let message = error.to_string();
    if message.contains("users_nickname_key") {
        return InsertUserError::Conflict(UserConflict::Nickname);
    }
    if message.contains("users_email_normalized_key") {
        return InsertUserError::Conflict(UserConflict::Email);
    }

    InsertUserError::Database(error)
}
