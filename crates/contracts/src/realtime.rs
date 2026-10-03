//! Общие контракты realtime WebTransport.

mod control;
mod envelope;
mod network;
mod server;
mod social;
mod text_chat;
mod typing;
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
    ConversationReadCheckpoint, DirectMessageCreated, DirectMessageTypingChanged,
    DirectMessageTypingSnapshot, DirectMessageTypingSnapshotRequest, SocialChangeReason,
    SocialChanged, SocialKind, SocialReady, StartDirectMessageTyping, StopDirectMessageTyping,
    SubscribeSocial,
};
pub use text_chat::{
    ChatImageLoadedResponse, ChatImageUploadResponse, DeleteMessage, DeleteMessageAccepted,
    LoadChatImage, LoadRoomHistory, MessageDeletedPayload, RoomHistory, SendMessage,
    SendMessageAccepted, StartTyping, StopTyping, TextChatImageAttachment, TextChatKind,
    TextChatMessage, TypingChanged, TypingSnapshot, TypingSnapshotRequest, UploadChatImage,
};
pub use typing::TypingAuthor;
pub use voice_chat::{
    BindMicrophoneUplink, CancelDirectCall, DirectCallEndReason, DirectCallLifecycleEvent,
    DirectCallResponse, DirectCallSnapshot, DirectCallState, DirectCallsSnapshot,
    DirectMessageVoiceRoomsSnapshot, EndDirectCall, IssueMicrophoneUplinkGrant,
    JoinDirectMessageVoiceRoom, JoinVoiceRoom, KickVoiceMember, LeaveDirectMessageVoiceRoom,
    LeaveVoiceRoom, ListDirectCalls, ListDirectMessageVoiceRooms, ListServerVoiceRooms,
    MicrophoneUplinkBound, MicrophoneUplinkGrantIssued, ParticipantNetworkQualityUpdated,
    PublishVoiceNetworkQuality, RespondDirectCall, ServerAudioBitrate, ServerVoiceRoomsSnapshot,
    StartDirectCall, StopVoiceVideoStream, VoiceChatKind, VoiceNetworkTargetKind,
    VoiceRoomParticipant, VoiceRoomSnapshot, VoiceVideoStreamEnded, VoiceVideoStreamSource,
};

#[cfg(test)]
mod tests;
