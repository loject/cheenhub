//! Потоки настройки системного автора сервера.

use cheenhub_contracts::rest::{ServerSystemAccountResponse, UpdateServerSystemAccountRequest};

use crate::features::servers::error::ServerError;
use crate::features::servers::validation::system_account_nickname;
use crate::state::AppState;

use super::support::{current_user_id, parse_server_id};

/// Возвращает системное отображаемое имя в настройках сервера.
pub(crate) async fn get_system_account(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<ServerSystemAccountResponse, ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let Some(profile) = state
        .server_store
        .find_server_system_account(&server_id, &owner_user_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        tracing::warn!(server_id = %server_id, owner_user_id = %owner_user_id, "rejected system account settings read");
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    };

    Ok(ServerSystemAccountResponse {
        nickname: profile.nickname,
    })
}

/// Сохраняет имя системного автора, доступное владельцу сервера.
pub(crate) async fn update_system_account(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: UpdateServerSystemAccountRequest,
) -> Result<ServerSystemAccountResponse, ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let nickname = system_account_nickname(request.nickname)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let Some(profile) = state
        .server_store
        .update_server_system_account_nickname(&server_id, &owner_user_id, nickname)
        .await
        .map_err(ServerError::Internal)?
    else {
        tracing::warn!(server_id = %server_id, owner_user_id = %owner_user_id, "rejected system account settings update");
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    };

    tracing::info!(server_id = %server_id, owner_user_id = %owner_user_id, "updated server system account name");
    Ok(ServerSystemAccountResponse {
        nickname: profile.nickname,
    })
}
