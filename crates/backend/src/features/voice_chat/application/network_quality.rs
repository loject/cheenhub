//! Публикация сетевых метрик участников голосового общения.

use cheenhub_contracts::realtime::{
    ParticipantNetworkQualityUpdated, PublishVoiceNetworkQuality, RealtimeKind, RealtimeModule,
    VoiceChatKind, VoiceNetworkTargetKind,
};
use std::time::Instant;
use uuid::Uuid;

use crate::features::voice_chat::infrastructure::{VoicePresenceTarget, VoicePresenceTargetKind};
use crate::state::AppState;

use super::VoiceChatApplicationError;

const MAX_RTT_MS: u32 = 5_000;

#[derive(Debug)]
pub(super) struct NetworkQualityBroadcast {
    pub(super) target: VoicePresenceTarget,
    pub(super) recipient_user_ids: Vec<Uuid>,
    pub(super) event: ParticipantNetworkQualityUpdated,
}

#[derive(Debug)]
pub(super) struct AuthorizedNetworkQualityPublication {
    target: VoicePresenceTarget,
    event: ParticipantNetworkQualityUpdated,
}

/// Публикует RTT аутентифицированного участника всем участникам его голосовой цели.
pub(crate) async fn publish_network_quality(
    state: &AppState,
    realtime_stream_id: Uuid,
    user_id: &Uuid,
    request: PublishVoiceNetworkQuality,
) -> Result<(), VoiceChatApplicationError> {
    let Some(publication) = authorize_network_quality_publication_at(
        state,
        realtime_stream_id,
        user_id,
        request,
        Instant::now(),
    )
    .await?
    else {
        return Ok(());
    };
    let broadcast = prepare_network_quality_broadcast(state, publication).await;
    let recipients = state
        .realtime_hub
        .recipients_for_users(RealtimeModule::VoiceChat, &broadcast.recipient_user_ids)
        .await;
    let stream_ids = recipients
        .iter()
        .map(|recipient| recipient.stream_id)
        .collect::<Vec<_>>();

    tracing::debug!(
        user_id = %user_id,
        target_kind = ?broadcast.target.kind,
        server_id = %broadcast.target.server_id,
        room_id = %broadcast.target.room_id,
        rtt_ms = broadcast.event.rtt_ms,
        recipients = stream_ids.len(),
        "broadcasting voice participant network quality"
    );
    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::VoiceChat,
            &broadcast.target.server_id,
            RealtimeKind::VoiceChat(VoiceChatKind::ParticipantNetworkQualityUpdated),
            &stream_ids,
            broadcast.event,
        )
        .await;
    Ok(())
}

pub(super) async fn authorize_network_quality_publication_at(
    state: &AppState,
    realtime_stream_id: Uuid,
    user_id: &Uuid,
    request: PublishVoiceNetworkQuality,
    now: Instant,
) -> Result<Option<AuthorizedNetworkQualityPublication>, VoiceChatApplicationError> {
    if request.rtt_ms > MAX_RTT_MS {
        tracing::warn!(
            user_id = %user_id,
            realtime_stream_id = %realtime_stream_id,
            rtt_ms = request.rtt_ms,
            "rejected invalid voice network quality value"
        );
        return Err(VoiceChatApplicationError::BadRequest(
            "Значение сетевой задержки некорректно.".to_owned(),
        ));
    }
    let presence = state
        .voice_presence_store
        .presence_for_stream(&realtime_stream_id, user_id)
        .await
        .ok_or_else(|| {
            tracing::warn!(
                user_id = %user_id,
                realtime_stream_id = %realtime_stream_id,
                "rejected voice network quality outside active presence"
            );
            VoiceChatApplicationError::Unauthorized(
                "Пользователь не участвует в голосовом общении.".to_owned(),
            )
        })?;
    let target = presence.target();
    if !state
        .voice_presence_store
        .allow_network_quality_publish_at(realtime_stream_id, now)
        .await
    {
        tracing::debug!(
            user_id = %user_id,
            realtime_stream_id = %realtime_stream_id,
            "dropped rate-limited voice network quality publication"
        );
        return Ok(None);
    }
    let target_kind = match target.kind {
        VoicePresenceTargetKind::Server => VoiceNetworkTargetKind::Server,
        VoicePresenceTargetKind::DirectMessage => VoiceNetworkTargetKind::DirectMessage,
    };

    Ok(Some(AuthorizedNetworkQualityPublication {
        target,
        event: ParticipantNetworkQualityUpdated {
            target_kind,
            server_id: target.server_id.to_string(),
            room_id: target.room_id.to_string(),
            user_id: user_id.to_string(),
            rtt_ms: request.rtt_ms,
        },
    }))
}

pub(super) async fn prepare_network_quality_broadcast(
    state: &AppState,
    publication: AuthorizedNetworkQualityPublication,
) -> NetworkQualityBroadcast {
    let recipient_user_ids = state
        .voice_presence_store
        .room_participants(
            publication.target.kind,
            &publication.target.server_id,
            &publication.target.room_id,
        )
        .await
        .into_iter()
        .map(|participant| participant.user_id)
        .collect();

    NetworkQualityBroadcast {
        target: publication.target,
        recipient_user_ids,
        event: publication.event,
    }
}
