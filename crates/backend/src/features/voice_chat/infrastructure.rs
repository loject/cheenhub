//! Voice chat presence infrastructure.

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

/// In-memory voice presence store for active realtime module streams.
#[derive(Default)]
pub(crate) struct InMemoryVoicePresenceStore {
    entries: Mutex<Vec<VoicePresence>>,
}

/// Active voice room presence row.
#[derive(Debug, Clone)]
pub(crate) struct VoicePresence {
    /// Realtime module stream that owns this presence and is used for disconnect cleanup.
    pub(crate) realtime_stream_id: Uuid,
    /// Server that contains the joined room.
    pub(crate) server_id: Uuid,
    /// Joined room identifier.
    pub(crate) room_id: Uuid,
    /// User that joined the room.
    pub(crate) user_id: Uuid,
    /// User nickname snapshot.
    pub(crate) nickname: String,
    /// Join timestamp.
    pub(crate) joined_at: DateTime<Utc>,
}

impl InMemoryVoicePresenceStore {
    /// Replaces presence for one user/realtime stream and returns removed entries.
    pub(crate) async fn join(&self, presence: VoicePresence) -> Vec<VoicePresence> {
        let mut entries = self.entries.lock().await;
        let mut removed = Vec::new();
        let realtime_stream_id = presence.realtime_stream_id;
        let user_id = presence.user_id;

        entries.retain(|entry| {
            let should_remove =
                entry.realtime_stream_id == realtime_stream_id || entry.user_id == user_id;
            if should_remove {
                removed.push(entry.clone());
            }
            !should_remove
        });
        entries.push(presence);

        removed
    }

    /// Removes presence for one realtime module stream.
    pub(crate) async fn leave_realtime_stream(
        &self,
        realtime_stream_id: &Uuid,
    ) -> Vec<VoicePresence> {
        self.remove_presence(|entry| &entry.realtime_stream_id == realtime_stream_id)
            .await
    }

    /// Removes presence for one realtime module stream in one room.
    pub(crate) async fn leave_room(
        &self,
        realtime_stream_id: &Uuid,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        self.remove_presence(|entry| {
            &entry.realtime_stream_id == realtime_stream_id
                && &entry.server_id == server_id
                && &entry.room_id == room_id
        })
        .await
    }

    async fn remove_presence(
        &self,
        should_remove: impl Fn(&VoicePresence) -> bool,
    ) -> Vec<VoicePresence> {
        let mut entries = self.entries.lock().await;
        let mut removed = Vec::new();

        entries.retain(|entry| {
            if should_remove(entry) {
                removed.push(entry.clone());
                false
            } else {
                true
            }
        });

        removed
    }

    /// Lists active participants for one room.
    pub(crate) async fn room_participants(
        &self,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        let mut participants = self
            .entries
            .lock()
            .await
            .iter()
            .filter(|entry| &entry.server_id == server_id && &entry.room_id == room_id)
            .cloned()
            .collect::<Vec<_>>();
        participants.sort_by_key(|presence| presence.joined_at);
        participants
    }
}
