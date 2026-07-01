//! Общие контракты realtime WebTransport.

mod control;
mod envelope;
mod network;
mod server;
mod social;
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
pub use social::{
    ConversationReadCheckpoint, SocialChangeReason, SocialChanged, SocialKind, SocialReady,
    SubscribeSocial,
};
pub use text_chat::{
    ChatImageLoadedResponse, ChatImageUploadResponse, DeleteMessage, DeleteMessageAccepted,
    LoadChatImage, LoadRoomHistory, MessageDeletedPayload, RoomHistory, SendMessage,
    SendMessageAccepted, TextChatImageAttachment, TextChatKind, TextChatMessage, UploadChatImage,
};
pub use voice_chat::{
    DirectMessageVoiceRoomsSnapshot, JoinDirectMessageVoiceRoom, JoinVoiceRoom, KickVoiceMember,
    LeaveDirectMessageVoiceRoom, LeaveVoiceRoom, ListDirectMessageVoiceRooms, ListServerVoiceRooms,
    ServerVoiceRoomsSnapshot, StopVoiceVideoStream, VoiceChatKind, VoiceRoomParticipant,
    VoiceRoomSnapshot, VoiceVideoStreamEnded, VoiceVideoStreamSource,
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
    fn voice_video_stream_ended_envelope_round_trips() {
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::VideoStreamEnded),
            None,
            VoiceVideoStreamEnded {
                server_id: Uuid::new_v4().to_string(),
                room_id: Uuid::new_v4().to_string(),
                user_id: Uuid::new_v4().to_string(),
                source: VoiceVideoStreamSource::ScreenShare,
            },
        )
        .expect("payload serializes");

        let json = serde_json::to_string(&envelope).expect("envelope serializes");
        assert!(json.contains("\"module\":\"voice_chat\""));
        assert!(json.contains("\"kind\":\"video_stream_ended\""));
        assert!(json.contains("\"source\":\"screen_share\""));
        let decoded: RealtimeEnvelope = serde_json::from_str(&json).expect("envelope decodes");

        assert_eq!(
            decoded.kind,
            RealtimeKind::VoiceChat(VoiceChatKind::VideoStreamEnded)
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
    fn social_changed_envelope_round_trips() {
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::Changed),
            None,
            SocialChanged {
                reason: SocialChangeReason::DirectMessages,
                conversation_id: Some(Uuid::new_v4().to_string()),
            },
        )
        .expect("envelope serializes");

        let json = serde_json::to_string(&envelope).expect("envelope serializes");
        assert!(json.contains("\"module\":\"social\""));
        assert!(json.contains("\"kind\":\"changed\""));
        let decoded: RealtimeEnvelope = serde_json::from_str(&json).expect("envelope decodes");

        assert_eq!(decoded.kind, RealtimeKind::Social(SocialKind::Changed));
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
            delivery_status: None,
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
