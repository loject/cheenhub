//! Voice chat presence realtime module contracts.

use serde::{Deserialize, Serialize};

/// Voice chat presence module message kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceChatKind {
    /// Join one voice-capable room.
    JoinVoiceRoom,
    /// Leave one voice-capable room.
    LeaveVoiceRoom,
    /// Current voice room participant snapshot.
    VoiceRoomSnapshot,
    /// Voice room participant list changed event.
    ParticipantsChanged,
}

/// Request payload used to join a voice-capable room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinVoiceRoom {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
}

/// Request payload used to leave a voice-capable room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveVoiceRoom {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
}

/// Current participants for one voice room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomSnapshot {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
    /// Participants currently present in the room.
    pub participants: Vec<VoiceRoomParticipant>,
}

/// Voice room participant payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomParticipant {
    /// Stable user identifier.
    pub user_id: String,
    /// User nickname snapshot.
    pub nickname: String,
    /// Public avatar image URL when configured.
    pub avatar_url: Option<String>,
    /// RFC3339 timestamp for when this participant joined.
    pub joined_at: String,
}
