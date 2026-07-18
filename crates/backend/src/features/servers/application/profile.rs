//! Server profile settings flows.

use bytes::Bytes;
use cheenhub_contracts::rest::{
    UpdateServerAvatarResponse, UpdateServerRequest, UpdateServerResponse,
};
use uuid::Uuid;

use crate::features::auth::error::AuthError;
use crate::features::images::application as image_application;
use crate::features::servers::error::ServerError;
use crate::features::servers::validation;
use crate::state::AppState;

use super::support::{current_user_id, parse_server_id, server_summary};

/// Updates a server profile owned by the current user.
pub(crate) async fn update(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: UpdateServerRequest,
) -> Result<UpdateServerResponse, ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let valid = validation::create_server(request.name)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let Some(server) = state
        .server_store
        .update_server_name(&server_id, &owner_user_id, valid.name)
        .await
        .map_err(ServerError::Internal)?
    else {
        tracing::warn!(
            server_id = %server_id,
            user_id = %owner_user_id,
            "rejected server profile update for non-owner or missing server"
        );
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    };

    tracing::info!(
        server_id = %server.id,
        owner_user_id = %owner_user_id,
        "updated server profile"
    );

    Ok(UpdateServerResponse {
        server: server_summary(state, &server, &owner_user_id, true).await,
    })
}

/// Updates a server avatar owned by the current user.
pub(crate) async fn update_avatar(
    state: &AppState,
    access_token: &str,
    server_id: String,
    bytes: Bytes,
) -> Result<UpdateServerAvatarResponse, ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let Some(server) = state
        .server_store
        .find_owned_server(&server_id, &owner_user_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        tracing::warn!(
            server_id = %server_id,
            user_id = %owner_user_id,
            "rejected server avatar update for non-owner or missing server"
        );
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    };

    tracing::debug!(
        server_id = %server.id,
        owner_user_id = %owner_user_id,
        input_bytes = bytes.len(),
        "processing server avatar upload"
    );
    let image_id = Uuid::new_v4();
    let processed = image_application::process_server_avatar(state, server.id, bytes.as_ref())
        .await
        .map_err(map_avatar_error)?;
    let byte_len = processed.byte_len();
    state
        .image_store
        .insert_image(processed.into_new_server_avatar_image(image_id, owner_user_id))
        .await
        .map_err(ServerError::Internal)?;
    let Some(server) = state
        .server_store
        .update_server_avatar_image_id(&server.id, &owner_user_id, image_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        tracing::warn!(
            server_id = %server_id,
            owner_user_id = %owner_user_id,
            image_id = %image_id,
            "server disappeared before avatar image reference update"
        );
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    };

    tracing::info!(
        server_id = %server.id,
        owner_user_id = %owner_user_id,
        image_id = %image_id,
        output_bytes = byte_len,
        "updated server avatar"
    );

    Ok(UpdateServerAvatarResponse {
        server: server_summary(state, &server, &owner_user_id, true).await,
    })
}

fn map_avatar_error(error: AuthError) -> ServerError {
    match error {
        AuthError::BadRequest(message) => ServerError::BadRequest(message),
        AuthError::Unauthorized(message) => ServerError::Unauthorized(message),
        AuthError::RefreshRejected { message, .. }
        | AuthError::RefreshRotationInProgress(message) => ServerError::Unauthorized(message),
        AuthError::Conflict(message) | AuthError::RateLimited(message) => {
            ServerError::BadRequest(message)
        }
        AuthError::Misconfigured { message, .. } => ServerError::Internal(anyhow::anyhow!(message)),
        AuthError::Internal(error) => ServerError::Internal(error),
    }
}
