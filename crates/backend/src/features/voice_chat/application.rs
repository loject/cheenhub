//! Voice chat presence application flows.

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, LeaveVoiceRoom, RealtimeKind, RealtimeModule, VoiceChatKind,
    VoiceRoomParticipant, VoiceRoomSnapshot,
};
use cheenhub_contracts::rest::{AuthUser, ServerRoomKind};
use chrono::Utc;
use uuid::Uuid;

use crate::features::voice_chat::infrastructure::VoicePresence;
use crate::state::AppState;

/// Joins one voice-capable room and returns the current participant snapshot.
pub(crate) async fn join_room(
    state: &AppState,
    realtime_stream_id: Uuid,
    session_id: Uuid,
    user: &AuthUser,
    user_id: &Uuid,
    request: JoinVoiceRoom,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    ensure_room_voice_available(state, user_id, &server_id, &room_id).await?;
    let removed = state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id,
            session_id,
            server_id,
            room_id,
            user_id: *user_id,
            nickname: user.nickname.clone(),
            joined_at: Utc::now(),
        })
        .await;

    fanout_removed_rooms(state, removed, Some((server_id, room_id))).await;
    let snapshot = room_snapshot(state, &server_id, &room_id).await;
    fanout_snapshot(state, snapshot.clone()).await;

    Ok(snapshot)
}

/// Leaves one voice-capable room and returns the current participant snapshot.
pub(crate) async fn leave_room(
    state: &AppState,
    realtime_stream_id: Uuid,
    user_id: &Uuid,
    request: LeaveVoiceRoom,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let removed = state
        .voice_presence_store
        .leave_room(&realtime_stream_id, &server_id, &room_id)
        .await;

    if removed.is_empty() {
        ensure_room_voice_available(state, user_id, &server_id, &room_id).await?;
        return Ok(room_snapshot(state, &server_id, &room_id).await);
    }

    let snapshot = room_snapshot(state, &server_id, &room_id).await;
    fanout_snapshot(state, snapshot.clone()).await;

    Ok(snapshot)
}

/// Removes presence owned by a closed realtime stream.
pub(crate) async fn disconnect_realtime_stream(state: &AppState, realtime_stream_id: Uuid) {
    let removed = state
        .voice_presence_store
        .leave_realtime_stream(&realtime_stream_id)
        .await;
    fanout_removed_rooms(state, removed, None).await;
}

/// Voice chat application error.
#[derive(Debug)]
pub(crate) enum VoiceChatApplicationError {
    /// Request shape or target is invalid.
    BadRequest(String),
    /// User cannot access the requested voice room.
    Unauthorized(String),
    /// Resource was not found.
    NotFound(String),
    /// Unexpected internal failure.
    Internal(anyhow::Error),
}

async fn ensure_room_voice_available(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
    room_id: &Uuid,
) -> Result<(), VoiceChatApplicationError> {
    let Some(room) = state
        .server_store
        .find_server_room(server_id, room_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
    else {
        return Err(VoiceChatApplicationError::NotFound(
            "Комната не найдена.".to_owned(),
        ));
    };
    if room.kind == ServerRoomKind::Text {
        return Err(VoiceChatApplicationError::BadRequest(
            "В этой комнате нет голосового чата.".to_owned(),
        ));
    }
    if user_has_server_access(state, user_id, server_id)
        .await
        .map_err(VoiceChatApplicationError::Internal)?
    {
        Ok(())
    } else {
        Err(VoiceChatApplicationError::Unauthorized(
            "Нет доступа к этой комнате.".to_owned(),
        ))
    }
}

async fn user_has_server_access(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(server) = state.server_store.find_server(server_id).await? else {
        return Ok(false);
    };
    if server.owner_user_id == *user_id {
        return Ok(true);
    }

    Ok(state
        .server_store
        .find_active_server_member(server_id, user_id)
        .await?
        .is_some())
}

async fn fanout_removed_rooms(
    state: &AppState,
    removed: Vec<VoicePresence>,
    skip: Option<(Uuid, Uuid)>,
) {
    let mut rooms = Vec::<(Uuid, Uuid)>::new();
    for presence in removed {
        let room = (presence.server_id, presence.room_id);
        if Some(room) == skip || rooms.contains(&room) {
            continue;
        }
        rooms.push(room);
    }

    for (server_id, room_id) in rooms {
        let snapshot = room_snapshot(state, &server_id, &room_id).await;
        fanout_snapshot(state, snapshot).await;
    }
}

async fn room_snapshot(state: &AppState, server_id: &Uuid, room_id: &Uuid) -> VoiceRoomSnapshot {
    let participants = state
        .voice_presence_store
        .room_participants(server_id, room_id)
        .await
        .iter()
        .map(participant_summary)
        .collect();

    VoiceRoomSnapshot {
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        participants,
    }
}

async fn fanout_snapshot(state: &AppState, snapshot: VoiceRoomSnapshot) {
    let Ok(server_id) = Uuid::parse_str(&snapshot.server_id) else {
        return;
    };
    let recipients = state
        .realtime_hub
        .recipients(state, RealtimeModule::VoiceChat, &server_id)
        .await;
    let stream_ids = recipients
        .iter()
        .map(|recipient| recipient.stream_id)
        .collect::<Vec<_>>();

    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::VoiceChat,
            &server_id,
            RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged),
            &stream_ids,
            snapshot,
        )
        .await;
}

fn parse_id(value: &str, message: &str) -> Result<Uuid, VoiceChatApplicationError> {
    Uuid::parse_str(value).map_err(|_| VoiceChatApplicationError::BadRequest(message.to_owned()))
}

fn participant_summary(presence: &VoicePresence) -> VoiceRoomParticipant {
    VoiceRoomParticipant {
        user_id: presence.user_id.to_string(),
        nickname: presence.nickname.clone(),
        joined_at: presence.joined_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cheenhub_contracts::realtime::{JoinVoiceRoom, LeaveVoiceRoom};
    use cheenhub_contracts::rest::{RegisterRequest, ServerRoomKind};

    use super::{VoiceChatApplicationError, join_room, leave_room};
    use crate::features::auth::application as auth_application;
    use crate::features::auth::infrastructure::InMemoryAuthStore;
    use crate::features::auth::security::keys::AuthKeys;
    use crate::features::servers::infrastructure::InMemoryServerStore;
    use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
    use crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore;
    use crate::realtime::hub::RealtimeHub;
    use crate::state::AppState;

    fn state() -> AppState {
        AppState {
            auth_store: Arc::new(InMemoryAuthStore::default()),
            auth_mailer: Arc::new(crate::features::auth::email::tests::TestAuthMailer::default()),
            server_store: Arc::new(InMemoryServerStore::default()),
            text_chat_store: Arc::new(InMemoryTextChatStore::default()),
            voice_presence_store: Arc::new(InMemoryVoicePresenceStore::default()),
            realtime_hub: Arc::new(RealtimeHub::default()),
            auth_keys: AuthKeys::generate_for_tests(),
            access_token_lifetime_minutes: 15,
            refresh_token_lifetime_days: 30,
            google_oauth_client_id: Some("test-google-client".to_owned()),
            google_oauth_client_secret: Some("test-google-secret".to_owned()),
            google_oauth_redirect_uri: Some(
                "http://localhost/api/auth/oauth/google/callback".to_owned(),
            ),
            cheenhub_client_base_url: "http://localhost".to_owned(),
            oauth_state_lifetime_minutes: 10,
            oauth_handoff_lifetime_minutes: 5,
            oauth_registration_lifetime_minutes: 15,
            password_reset_token_lifetime_minutes: 30,
        }
    }

    async fn registered_user(state: &AppState) -> (cheenhub_contracts::rest::AuthUser, uuid::Uuid) {
        let auth = auth_application::register(
            state,
            RegisterRequest {
                nickname: "voice_owner".to_owned(),
                email: "voice-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("registration should succeed");
        let user_id = uuid::Uuid::parse_str(&auth.user.id).expect("user id should be uuid");

        (auth.user, user_id)
    }

    async fn create_room(
        state: &AppState,
        user_id: &uuid::Uuid,
        room_name: &str,
        kind: ServerRoomKind,
    ) -> (String, String) {
        let server = state
            .server_store
            .insert_server(user_id, "Voice Server".to_owned())
            .await
            .expect("server should insert");
        state
            .server_store
            .insert_server_member(&server.id, user_id)
            .await
            .expect("member should insert");
        let room = state
            .server_store
            .insert_server_room(&server.id, room_name.to_owned(), kind)
            .await
            .expect("room should insert");

        (server.id.to_string(), room.id.to_string())
    }

    #[tokio::test]
    async fn user_can_join_and_leave_voice_room() {
        let state = state();
        let (user, user_id) = registered_user(&state).await;
        let stream_id = uuid::Uuid::new_v4();
        let (server_id, room_id) =
            create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;

        let snapshot = join_room(
            &state,
            stream_id,
            uuid::Uuid::new_v4(),
            &user,
            &user_id,
            JoinVoiceRoom {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
            },
        )
        .await
        .expect("join should succeed");

        assert_eq!(snapshot.participants.len(), 1);
        assert_eq!(snapshot.participants[0].nickname, "voice_owner");

        let snapshot = leave_room(
            &state,
            stream_id,
            &user_id,
            LeaveVoiceRoom { server_id, room_id },
        )
        .await
        .expect("leave should succeed");

        assert!(snapshot.participants.is_empty());
    }

    #[tokio::test]
    async fn text_room_rejects_voice_join() {
        let state = state();
        let (user, user_id) = registered_user(&state).await;
        let (server_id, room_id) =
            create_room(&state, &user_id, "text", ServerRoomKind::Text).await;

        let error = join_room(
            &state,
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            &user,
            &user_id,
            JoinVoiceRoom { server_id, room_id },
        )
        .await
        .expect_err("text rooms should reject voice presence");

        assert!(matches!(error, VoiceChatApplicationError::BadRequest(_)));
    }

    #[tokio::test]
    async fn user_can_leave_after_room_becomes_text_only() {
        let state = state();
        let (user, user_id) = registered_user(&state).await;
        let stream_id = uuid::Uuid::new_v4();
        let (server_id, room_id) =
            create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;

        join_room(
            &state,
            stream_id,
            uuid::Uuid::new_v4(),
            &user,
            &user_id,
            JoinVoiceRoom {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
            },
        )
        .await
        .expect("join should succeed");

        let server_uuid = uuid::Uuid::parse_str(&server_id).expect("server id");
        let room_uuid = uuid::Uuid::parse_str(&room_id).expect("room id");
        state
            .server_store
            .update_server_room(
                &server_uuid,
                &room_uuid,
                "voice".to_owned(),
                ServerRoomKind::Text,
            )
            .await
            .expect("room update should succeed");

        let snapshot = leave_room(
            &state,
            stream_id,
            &user_id,
            LeaveVoiceRoom { server_id, room_id },
        )
        .await
        .expect("leave should remove existing stale presence");

        assert!(snapshot.participants.is_empty());
    }

    #[tokio::test]
    async fn joining_new_room_replaces_previous_presence() {
        let state = state();
        let (user, user_id) = registered_user(&state).await;
        let stream_id = uuid::Uuid::new_v4();
        let (first_server_id, first_room_id) =
            create_room(&state, &user_id, "first", ServerRoomKind::Voice).await;
        let (second_server_id, second_room_id) =
            create_room(&state, &user_id, "second", ServerRoomKind::TextAndVoice).await;

        join_room(
            &state,
            stream_id,
            uuid::Uuid::new_v4(),
            &user,
            &user_id,
            JoinVoiceRoom {
                server_id: first_server_id.clone(),
                room_id: first_room_id.clone(),
            },
        )
        .await
        .expect("first join should succeed");
        let snapshot = join_room(
            &state,
            stream_id,
            uuid::Uuid::new_v4(),
            &user,
            &user_id,
            JoinVoiceRoom {
                server_id: second_server_id.clone(),
                room_id: second_room_id.clone(),
            },
        )
        .await
        .expect("second join should succeed");

        assert_eq!(snapshot.server_id, second_server_id);
        assert_eq!(snapshot.room_id, second_room_id);
        assert_eq!(snapshot.participants.len(), 1);

        let first_server = uuid::Uuid::parse_str(&first_server_id).expect("server id");
        let first_room = uuid::Uuid::parse_str(&first_room_id).expect("room id");
        assert!(
            state
                .voice_presence_store
                .room_participants(&first_server, &first_room)
                .await
                .is_empty()
        );
    }
}
