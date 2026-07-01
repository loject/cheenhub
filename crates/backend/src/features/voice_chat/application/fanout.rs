//! Рассылка снимков голосового присутствия участникам комнат.

use cheenhub_contracts::realtime::{
    RealtimeKind, RealtimeModule, VoiceChatKind, VoiceRoomParticipant, VoiceRoomSnapshot,
};
use uuid::Uuid;

use crate::features::social::{self, SocialError};
use crate::features::voice_chat::infrastructure::{
    VoicePresence, VoicePresenceTarget, VoicePresenceTargetKind,
};
use crate::state::AppState;

pub(super) async fn fanout_removed_rooms(
    state: &AppState,
    removed: Vec<VoicePresence>,
    skip: Option<VoicePresenceTarget>,
) {
    let mut rooms = Vec::<VoicePresenceTarget>::new();
    for presence in removed {
        let room = presence.target();
        if Some(room) == skip || rooms.contains(&room) {
            continue;
        }
        rooms.push(room);
    }

    for target in rooms {
        let snapshot = room_snapshot(state, target).await;
        fanout_snapshot(state, target, snapshot).await;
    }
}

pub(super) async fn room_snapshot(
    state: &AppState,
    target: VoicePresenceTarget,
) -> VoiceRoomSnapshot {
    let participants = state
        .voice_presence_store
        .room_participants(target.kind, &target.server_id, &target.room_id)
        .await
        .iter()
        .map(participant_summary)
        .collect();

    VoiceRoomSnapshot {
        server_id: target.route_id().to_string(),
        room_id: target.room_id.to_string(),
        participants,
    }
}

pub(super) async fn fanout_snapshot(
    state: &AppState,
    target: VoicePresenceTarget,
    snapshot: VoiceRoomSnapshot,
) {
    let stream_ids = match target.kind {
        VoicePresenceTargetKind::Server => state
            .realtime_hub
            .recipients(state, RealtimeModule::VoiceChat, &target.server_id)
            .await
            .iter()
            .map(|recipient| recipient.stream_id)
            .collect::<Vec<_>>(),
        VoicePresenceTargetKind::DirectMessage => {
            let user_ids = match direct_message_user_ids(state, &target.room_id).await {
                Ok(user_ids) => user_ids,
                Err(error) => {
                    tracing::warn!(
                        conversation_id = %target.room_id,
                        %error,
                        "failed to evaluate direct message voice recipients"
                    );
                    return;
                }
            };
            state
                .realtime_hub
                .recipients_for_users(RealtimeModule::VoiceChat, &user_ids)
                .await
                .iter()
                .map(|recipient| recipient.stream_id)
                .collect::<Vec<_>>()
        }
    };
    tracing::debug!(
        server_id = %snapshot.server_id,
        room_id = %snapshot.room_id,
        target_kind = ?target.kind,
        participants = snapshot.participants.len(),
        recipients = stream_ids.len(),
        "fanning out voice room participants changed event"
    );

    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::VoiceChat,
            &target.route_id(),
            RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged),
            &stream_ids,
            snapshot,
        )
        .await;
}

pub(super) fn server_voice_target(server_id: Uuid, room_id: Uuid) -> VoicePresenceTarget {
    VoicePresenceTarget {
        kind: VoicePresenceTargetKind::Server,
        server_id,
        room_id,
    }
}

pub(super) fn direct_message_voice_target(conversation_id: Uuid) -> VoicePresenceTarget {
    VoicePresenceTarget {
        kind: VoicePresenceTargetKind::DirectMessage,
        server_id: conversation_id,
        room_id: conversation_id,
    }
}

async fn direct_message_user_ids(
    state: &AppState,
    conversation_id: &Uuid,
) -> anyhow::Result<Vec<Uuid>> {
    match social::direct_message_voice_user_ids(state, conversation_id).await {
        Ok(user_ids) => Ok(user_ids),
        Err(SocialError::Internal(error)) => Err(error),
        Err(error) => {
            tracing::warn!(
                conversation_id = %conversation_id,
                ?error,
                "direct message voice recipients are unavailable"
            );
            Ok(Vec::new())
        }
    }
}

pub(super) fn participant_summary(presence: &VoicePresence) -> VoiceRoomParticipant {
    VoiceRoomParticipant {
        user_id: presence.user_id.to_string(),
        nickname: presence.nickname.clone(),
        avatar_url: presence.avatar_url.clone(),
        joined_at: presence.joined_at.to_rfc3339(),
    }
}
