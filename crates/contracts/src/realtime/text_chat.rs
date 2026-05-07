//! Text chat realtime module contracts.

use serde::{Deserialize, Serialize};

/// Text chat module message kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextChatKind {
    /// Load the latest room message history.
    LoadRoomHistory,
    /// Room message history response.
    RoomHistory,
    /// Send a message to a room.
    SendMessage,
    /// Acknowledges that a message send was accepted for fanout and persistence.
    SendMessageAccepted,
    /// A newly created message event.
    MessageCreated,
}

/// Request payload used to load room history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRoomHistory {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
}

/// Response payload containing the latest room messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomHistory {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
    /// Latest persisted room messages.
    pub messages: Vec<TextChatMessage>,
}

/// Request payload used to send a room message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessage {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
    /// Message body.
    pub body: String,
}

/// Response payload returned after accepting a message send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessageAccepted {
    /// Accepted message.
    pub message: TextChatMessage,
}

/// Text chat message payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextChatMessage {
    /// Stable message identifier.
    pub id: String,
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
    /// Author user identifier.
    pub author_user_id: String,
    /// Author nickname snapshot.
    pub author_nickname: String,
    /// Message body.
    pub body: String,
    /// Message creation timestamp in RFC3339 format.
    pub created_at: String,
}
