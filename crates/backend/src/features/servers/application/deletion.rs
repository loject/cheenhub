//! Удаление сервера вместе с его содержимым.

use crate::features::servers::error::ServerError;
use crate::state::AppState;

use super::support::{current_user_id, parse_server_id};

/// Удаляет сервер, принадлежащий текущему пользователю.
///
/// Удаление необратимо: вместе с сервером пропадают его комнаты, сообщения,
/// приглашения, участники и роли. Поэтому сервер ищется по паре «идентификатор —
/// владелец», и любой другой пользователь получает тот же отказ «сервер не найден»,
/// что и для несуществующего сервера: чужие серверы нельзя даже обнаружить.
pub(crate) async fn delete(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<(), ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;

    if !state
        .server_store
        .delete_owned_server(&server_id, &owner_user_id)
        .await
        .map_err(ServerError::Internal)?
    {
        tracing::warn!(
            %server_id,
            user_id = %owner_user_id,
            "rejected server deletion for non-owner or missing server"
        );
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    }

    crate::features::voice_chat::application::remove_deleted_server(state, server_id).await;

    tracing::info!(
        %server_id,
        owner_user_id = %owner_user_id,
        "deleted server with its rooms, members and invites"
    );

    Ok(())
}
