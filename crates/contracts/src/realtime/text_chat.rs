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
    /// Delete one of the user's own messages.
    DeleteMessage,
    /// Acknowledges that a message deletion was accepted.
    DeleteMessageAccepted,
    /// A message was deleted by its author; recipients should remove it.
    MessageDeleted,
}

/// Request payload used to load room history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRoomHistory {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
    /// Message identifier to load messages before.
    pub before_message_id: Option<String>,
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
    /// Whether older messages are available before this page.
    pub has_more: bool,
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

/// Request payload used to soft-delete one of the user's own messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteMessage {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
    /// Identifier of the message to delete.
    pub message_id: String,
}

/// Response payload returned after accepting a message deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteMessageAccepted {
    /// Identifier of the deleted message.
    pub message_id: String,
}

/// Broadcast payload notifying room members that a message was removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeletedPayload {
    /// Server identifier.
    pub server_id: String,
    /// Room identifier.
    pub room_id: String,
    /// Identifier of the removed message.
    pub message_id: String,
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
    /// Public author avatar image URL when configured.
    pub author_avatar_url: Option<String>,
    /// Message body.
    pub body: String,
    /// Message creation timestamp in RFC3339 format.
    pub created_at: String,
}
