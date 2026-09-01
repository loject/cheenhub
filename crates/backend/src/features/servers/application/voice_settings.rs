//! Настройки качества голоса сервера.

use cheenhub_contracts::media::{VOICE_AUDIO_BITRATE_MAX_BPS, VOICE_AUDIO_BITRATE_MIN_BPS};
use cheenhub_contracts::rest::ServerVoiceSettings;

use crate::features::servers::error::ServerError;
use crate::state::AppState;

use super::support::{current_user_id, owned_server, parse_server_id, server_for_member_or_owner};

/// Возвращает настройки качества голоса сервера для участника или владельца.
pub(crate) async fn get_voice_settings(
    state: &AppState,
    access_token: &str,
    server_id: String,
) -> Result<ServerVoiceSettings, ServerError> {
    let user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let server = server_for_member_or_owner(state, &server_id, &user_id).await?;

    Ok(ServerVoiceSettings {
        audio_bitrate_bps: server.audio_bitrate_bps,
    })
}

/// Обновляет настройки качества голоса сервера, принадлежащего текущему пользователю.
pub(crate) async fn update_voice_settings(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: ServerVoiceSettings,
) -> Result<ServerVoiceSettings, ServerError> {
    let owner_user_id = current_user_id(state, access_token).await?;
    let server_id = parse_server_id(server_id)?;
    let server = owned_server(state, &server_id, &owner_user_id).await?;

    if !(VOICE_AUDIO_BITRATE_MIN_BPS..=VOICE_AUDIO_BITRATE_MAX_BPS)
        .contains(&request.audio_bitrate_bps)
    {
        return Err(ServerError::BadRequest(
            "Битрейт должен быть в диапазоне от 16 до 96 кбит/с.".to_owned(),
        ));
    }

    let updated = state
        .server_store
        .update_server_audio_bitrate(&server.id, &owner_user_id, request.audio_bitrate_bps)
        .await
        .map_err(ServerError::Internal)?
        .ok_or_else(|| ServerError::NotFound("Сервер не найден или недоступен.".to_owned()))?;

    tracing::info!(
        server_id = %updated.id,
        user_id = %owner_user_id,
        audio_bitrate_bps = updated.audio_bitrate_bps,
        "updated server voice settings"
    );

    crate::features::voice_chat::application::broadcast_server_audio_bitrate(
        state,
        &updated.id,
        updated.audio_bitrate_bps,
    )
    .await;

    Ok(ServerVoiceSettings {
        audio_bitrate_bps: updated.audio_bitrate_bps,
    })
}
