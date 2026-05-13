//! Postgres user storage helpers.

use crate::features::auth::infrastructure::{InsertUserError, UserConflict};

/// Maps database insert errors to user field conflicts.
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
