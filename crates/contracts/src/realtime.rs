//! Shared realtime WebTransport contracts.

mod control;
mod envelope;
mod network;
mod server;
mod text_chat;
mod voice_chat;

pub use control::{
    Authenticate, Authenticated, ControlAck, ControlKind, ControlText, Rejected, RejectionCode,
};
pub use envelope::{RealtimeEnvelope, RealtimeKind, RealtimeModule};
pub use network::{NetworkKind, Ping, Pong};
pub use server::{
    AssignServerMemberRole, KickServerInviteMember, KickServerMember, ListServerInvites,
    ListServerMembers, ListServerRoles, RevokeServerInvite, RevokeServerMemberRole,
    SaveServerRoles, ServerInviteJoinedMember, ServerInviteLink, ServerInviteList,
    ServerInviteMemberKicked, ServerInviteRevoked, ServerKind, ServerMemberEntry,
    ServerMemberKicked, ServerMemberList, ServerMemberRoleAssigned, ServerMemberRoleRevoked,
    ServerRoleDraft, ServerRoleEntry, ServerRoleKind, ServerRoleList, ServerRolePermission,
    ServerRoleSummary, ServerRolesSaved,
};
pub use text_chat::{
    ChatImageLoadedResponse, ChatImageUploadResponse, DeleteMessage, DeleteMessageAccepted,
    LoadChatImage, LoadRoomHistory, MessageDeletedPayload, RoomHistory, SendMessage,
    SendMessageAccepted, TextChatImageAttachment, TextChatKind, TextChatMessage, UploadChatImage,
};
pub use voice_chat::{
    JoinVoiceRoom, KickVoiceMember, LeaveVoiceRoom, ListServerVoiceRooms, ServerVoiceRoomsSnapshot,
    VoiceChatKind, VoiceRoomParticipant, VoiceRoomSnapshot,
};

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn envelope_round_trips_uuid_and_typed_kind() {
        let request_id = Uuid::new_v4();
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::Network,
            RealtimeKind::Network(NetworkKind::Ping),
            Some(request_id),
            Ping { sent_at_ms: 42 },
        )
        .expect("payload serializes");

        let json = serde_json::to_string(&envelope).expect("envelope serializes");
        assert!(json.contains("\"module\":\"network\""));
        assert!(json.contains("\"kind\":\"ping\""));
        let decoded: RealtimeEnvelope = serde_json::from_str(&json).expect("envelope decodes");

        assert_eq!(decoded.request_id, Some(request_id));
        assert_eq!(decoded.kind, RealtimeKind::Network(NetworkKind::Ping));
        assert!(decoded.has_matching_module_kind());
    }

    #[test]
    fn module_kind_mismatch_is_detected() {
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::Control,
            RealtimeKind::Network(NetworkKind::Ping),
            None,
            Ping { sent_at_ms: 42 },
        )
        .expect("payload serializes");

        assert!(!envelope.has_matching_module_kind());
    }

    #[test]
    fn text_chat_envelope_round_trips() {
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::LoadRoomHistory),
            Some(Uuid::new_v4()),
            LoadRoomHistory {
                server_id: Uuid::new_v4().to_string(),
                room_id: Uuid::new_v4().to_string(),
                before_message_id: None,
            },
        )
        .expect("payload serializes");

        let json = serde_json::to_string(&envelope).expect("envelope serializes");
        assert!(json.contains("\"module\":\"text_chat\""));
        assert!(json.contains("\"kind\":\"load_room_history\""));
        let decoded: RealtimeEnvelope = serde_json::from_str(&json).expect("envelope decodes");

        assert_eq!(
            decoded.kind,
            RealtimeKind::TextChat(TextChatKind::LoadRoomHistory)
        );
        assert!(decoded.has_matching_module_kind());
    }

    #[test]
    fn server_invites_envelope_round_trips() {
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::Server,
            RealtimeKind::Server(ServerKind::ListServerInvites),
            Some(Uuid::new_v4()),
            ListServerInvites {
                server_id: Uuid::new_v4().to_string(),
            },
        )
        .expect("payload serializes");

        let json = serde_json::to_string(&envelope).expect("envelope serializes");
        assert!(json.contains("\"module\":\"server\""));
        assert!(json.contains("\"kind\":\"list_server_invites\""));
        let decoded: RealtimeEnvelope = serde_json::from_str(&json).expect("envelope decodes");

        assert_eq!(
            decoded.kind,
            RealtimeKind::Server(ServerKind::ListServerInvites)
        );
        assert!(decoded.has_matching_module_kind());
    }

    #[test]
    fn avatar_fields_round_trip_in_realtime_payloads() {
        let message = TextChatMessage {
            id: Uuid::new_v4().to_string(),
            server_id: Uuid::new_v4().to_string(),
            room_id: Uuid::new_v4().to_string(),
            author_user_id: Uuid::new_v4().to_string(),
            author_nickname: "avatar_user".to_owned(),
            author_avatar_url: Some("http://localhost/api/images/avatar".to_owned()),
            body: "hello".to_owned(),
            attachments: Vec::new(),
            created_at: "2026-05-13T00:00:00Z".to_owned(),
        };
        let decoded: TextChatMessage =
            serde_json::from_str(&serde_json::to_string(&message).expect("message serializes"))
                .expect("message decodes");
        assert_eq!(decoded.author_avatar_url, message.author_avatar_url);

        let participant = VoiceRoomParticipant {
            user_id: Uuid::new_v4().to_string(),
            nickname: "voice_user".to_owned(),
            avatar_url: Some("http://localhost/api/images/avatar".to_owned()),
            joined_at: "2026-05-13T00:00:00Z".to_owned(),
        };
        let decoded: VoiceRoomParticipant = serde_json::from_str(
            &serde_json::to_string(&participant).expect("participant serializes"),
        )
        .expect("participant decodes");
        assert_eq!(decoded.avatar_url, participant.avatar_url);
    }
}
