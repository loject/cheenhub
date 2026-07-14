//! Выдача и потребление разрешений отдельной сессии отправки микрофона.

use cheenhub_contracts::realtime::{
    BindMicrophoneUplink, IssueMicrophoneUplinkGrant, MicrophoneUplinkBound,
    MicrophoneUplinkGrantIssued,
};
use chrono::{Duration, Utc};
use tracing::{info, warn};
use uuid::Uuid;

use super::{VoiceChatApplicationError, parse_id};
use crate::features::voice_chat::application::presence::active_presence_for_user;
use crate::features::voice_chat::infrastructure::{
    ConsumeMicrophoneUplinkGrantError, MicrophoneUplinkGrant,
};
use crate::state::AppState;

const MICROPHONE_UPLINK_GRANT_LIFETIME_SECONDS: i64 = 20;

/// Выдает основной сессии одноразовый grant для отдельной отправки микрофона.
pub(crate) async fn issue_microphone_uplink_grant(
    state: &AppState,
    session_id: Uuid,
    user_id: &Uuid,
    request: IssueMicrophoneUplinkGrant,
) -> Result<MicrophoneUplinkGrantIssued, VoiceChatApplicationError> {
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let Some(presence) = active_presence_for_user(state, &room_id, user_id).await else {
        warn!(%session_id, %user_id, %room_id, "отклонена выдача microphone uplink grant вне активной комнаты");
        return Err(VoiceChatApplicationError::Unauthorized(
            "Сначала войдите в голосовую комнату.".to_owned(),
        ));
    };
    if presence.session_id != session_id {
        warn!(
            %session_id,
            expected_session_id = %presence.session_id,
            %user_id,
            %room_id,
            "отклонена выдача microphone uplink grant устаревшей сессии"
        );
        return Err(VoiceChatApplicationError::Unauthorized(
            "Текущая сессия не владеет голосовым присутствием.".to_owned(),
        ));
    }

    let now = Utc::now();
    let expires_at = now + Duration::seconds(MICROPHONE_UPLINK_GRANT_LIFETIME_SECONDS);
    let grant_id = Uuid::new_v4();
    state
        .voice_presence_store
        .issue_microphone_uplink_grant(MicrophoneUplinkGrant {
            id: grant_id,
            user_id: *user_id,
            room_id,
            presence_session_id: session_id,
            expires_at,
        })
        .await;
    info!(%session_id, %user_id, %room_id, %expires_at, "выдан одноразовый microphone uplink grant");

    Ok(MicrophoneUplinkGrantIssued {
        grant: grant_id.to_string(),
        room_id: room_id.to_string(),
        expires_at: expires_at.to_rfc3339(),
    })
}

/// Потребляет grant и привязывает текущую сессию к отправке микрофона.
pub(crate) async fn bind_microphone_uplink(
    state: &AppState,
    session_id: Uuid,
    user_id: &Uuid,
    request: BindMicrophoneUplink,
) -> Result<MicrophoneUplinkBound, VoiceChatApplicationError> {
    let grant_id = parse_id(
        &request.grant,
        "Разрешение отправки микрофона недействительно.",
    )?;
    let binding = state
        .voice_presence_store
        .consume_microphone_uplink_grant(&grant_id, user_id, session_id, Utc::now())
        .await
        .map_err(|error| {
            let message = match error {
                ConsumeMicrophoneUplinkGrantError::Invalid => {
                    "Разрешение отправки микрофона недействительно."
                }
                ConsumeMicrophoneUplinkGrantError::Expired => {
                    "Срок действия разрешения отправки микрофона истек."
                }
            };
            warn!(%session_id, %user_id, ?error, "отклонена привязка microphone uplink");
            VoiceChatApplicationError::Unauthorized(message.to_owned())
        })?;

    let presence = active_presence_for_user(state, &binding.room_id, user_id).await;
    if presence.as_ref().map(|entry| entry.session_id) != Some(binding.presence_session_id) {
        warn!(
            %session_id,
            %user_id,
            room_id = %binding.room_id,
            "отклонена привязка microphone uplink без исходного присутствия"
        );
        return Err(VoiceChatApplicationError::Unauthorized(
            "Голосовое присутствие основной сессии уже завершено.".to_owned(),
        ));
    }

    info!(
        %session_id,
        presence_session_id = %binding.presence_session_id,
        %user_id,
        room_id = %binding.room_id,
        "привязана отдельная сессия отправки микрофона"
    );
    Ok(MicrophoneUplinkBound {
        room_id: binding.room_id.to_string(),
    })
}
