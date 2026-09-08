//! Публичные имена авторов истории с учётом удаления аккаунта.

use super::TextChatApplicationError;
use crate::features::text_chat::domain::TextMessage;
use crate::state::AppState;
use std::collections::HashMap;
use uuid::Uuid;

/// Проверяет удаление один раз для каждого автора страницы.
pub(super) async fn deleted_authors(
    state: &AppState,
    messages: &[TextMessage],
) -> Result<HashMap<Uuid, bool>, TextChatApplicationError> {
    let mut deleted = HashMap::new();
    for message in messages {
        if deleted.contains_key(&message.author_user_id) {
            continue;
        }
        let tombstone = state.auth_store.account_deletion(&message.author_user_id).await
            .map_err(|error| {
                tracing::error!(author_user_id = %message.author_user_id, %error, "failed to resolve text history author deletion");
                TextChatApplicationError::Internal(error)
            })?;
        deleted.insert(message.author_user_id, tombstone.is_some());
    }
    Ok(deleted)
}
