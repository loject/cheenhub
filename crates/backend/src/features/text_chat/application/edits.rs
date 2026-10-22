//! Редактирование текста собственных сообщений комнат.

use cheenhub_contracts::realtime::{EditMessage, EditMessageAccepted, TextChatKind};
use cheenhub_contracts::rest::AuthUser;
use uuid::Uuid;

use super::{
    TextChatApplicationError, ensure_room_write_allowed, fanout_room_message, message_summary,
    parse_id,
};
use crate::features::text_chat::validation;
use crate::state::AppState;

/// Изменяет текст сообщения автора и рассылает обновлённый снимок участникам комнаты.
///
/// Вложение остаётся неизменным; последняя запись текста выигрывает при гонке редакций.
pub(crate) async fn edit_message(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    request: EditMessage,
) -> Result<EditMessageAccepted, TextChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let message_id = parse_id(&request.message_id, "Сообщение не найдено.")?;

    ensure_room_write_allowed(state, user_id, &server_id, &room_id).await?;

    let existing = state
        .text_chat_store
        .find_editable_text_message(&server_id, &room_id, &message_id, user_id)
        .await
        .map_err(TextChatApplicationError::Internal)?
        .ok_or_else(|| {
            tracing::warn!(
                %server_id, %room_id, %message_id, %user_id,
                "rejected text message edit: not found, deleted or not author"
            );
            TextChatApplicationError::NotFound("Сообщение не найдено.".to_owned())
        })?;

    let body = if request.body.trim().is_empty() && !existing.attachments.is_empty() {
        String::new()
    } else {
        validation::message_body(request.body)
            .map_err(|message| TextChatApplicationError::BadRequest(message.to_owned()))?
            .body
    };

    let updated = state
        .text_chat_store
        .edit_text_message(&server_id, &room_id, &message_id, user_id, body)
        .await
        .map_err(TextChatApplicationError::Internal)?
        .ok_or_else(|| {
            TextChatApplicationError::NotFound("Сообщение не найдено или уже удалено.".to_owned())
        })?;

    let payload = message_summary(&updated, user.avatar_url.clone());
    tracing::debug!(%server_id, %room_id, %message_id, %user_id, "edited text chat message");

    if let Err(error) =
        fanout_room_message(state, payload.clone(), TextChatKind::MessageEdited).await
    {
        tracing::error!(
            %server_id, %room_id, %message_id, %user_id, %error,
            "failed to fan out edited text chat message"
        );
    }

    Ok(EditMessageAccepted { message: payload })
}
