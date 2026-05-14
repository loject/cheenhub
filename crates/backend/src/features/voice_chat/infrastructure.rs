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
    /// Authenticated WebTransport session that receives media datagrams.
    pub(crate) session_id: Uuid,
    /// Server that contains the joined room.
    pub(crate) server_id: Uuid,
    /// Joined room identifier.
    pub(crate) room_id: Uuid,
    /// User that joined the room.
    pub(crate) user_id: Uuid,
    /// User nickname snapshot.
    pub(crate) nickname: String,
    /// Public avatar URL snapshot.
    pub(crate) avatar_url: Option<String>,
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

    /// Returns the active presence for one user in one room.
    pub(crate) async fn room_presence_for_user(
        &self,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<VoicePresence> {
        self.entries
            .lock()
            .await
            .iter()
            .find(|entry| &entry.room_id == room_id && &entry.user_id == user_id)
            .cloned()
    }

    /// Updates active presence nickname rows for one user and returns affected room identifiers.
    pub(crate) async fn update_user_nickname(
        &self,
        user_id: &Uuid,
        nickname: String,
    ) -> Vec<(Uuid, Uuid)> {
        let mut entries = self.entries.lock().await;
        let mut rooms = Vec::<(Uuid, Uuid)>::new();

        for entry in entries.iter_mut().filter(|entry| &entry.user_id == user_id) {
            entry.nickname = nickname.clone();
            let room = (entry.server_id, entry.room_id);
            if !rooms.contains(&room) {
                rooms.push(room);
            }
        }

        rooms
    }

    /// Updates active presence avatar URLs for one user and returns affected room identifiers.
    pub(crate) async fn update_user_avatar(
        &self,
        user_id: &Uuid,
        avatar_url: Option<String>,
    ) -> Vec<(Uuid, Uuid)> {
        let mut entries = self.entries.lock().await;
        let mut rooms = Vec::<(Uuid, Uuid)>::new();

        for entry in entries.iter_mut().filter(|entry| &entry.user_id == user_id) {
            entry.avatar_url = avatar_url.clone();
            let room = (entry.server_id, entry.room_id);
            if !rooms.contains(&room) {
                rooms.push(room);
            }
        }

        rooms
    }

    /// Lists active media recipients in one room excluding one sender session.
    pub(crate) async fn media_recipient_sessions(
        &self,
        room_id: &Uuid,
        sender_session_id: &Uuid,
    ) -> Vec<Uuid> {
        self.entries
            .lock()
            .await
            .iter()
            .filter(|entry| &entry.room_id == room_id && &entry.session_id != sender_session_id)
            .map(|entry| entry.session_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::{InMemoryVoicePresenceStore, VoicePresence};

    fn presence(
        realtime_stream_id: Uuid,
        session_id: Uuid,
        server_id: Uuid,
        room_id: Uuid,
        user_id: Uuid,
    ) -> VoicePresence {
        VoicePresence {
            realtime_stream_id,
            session_id,
            server_id,
            room_id,
            user_id,
            nickname: "voice_user".to_owned(),
            avatar_url: None,
            joined_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn room_presence_authorizes_only_joined_users() {
        let store = InMemoryVoicePresenceStore::default();
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        assert!(
            store
                .room_presence_for_user(&room_id, &user_id)
                .await
                .is_none()
        );

        store
            .join(presence(
                Uuid::new_v4(),
                Uuid::new_v4(),
                Uuid::new_v4(),
                room_id,
                user_id,
            ))
            .await;

        assert!(
            store
                .room_presence_for_user(&room_id, &user_id)
                .await
                .is_some()
        );
    }

    #[tokio::test]
    async fn media_recipients_exclude_sender_and_other_rooms() {
        let store = InMemoryVoicePresenceStore::default();
        let server_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let other_room_id = Uuid::new_v4();
        let sender_session_id = Uuid::new_v4();
        let recipient_session_id = Uuid::new_v4();
        let other_room_session_id = Uuid::new_v4();

        store
            .join(presence(
                Uuid::new_v4(),
                sender_session_id,
                server_id,
                room_id,
                Uuid::new_v4(),
            ))
            .await;
        store
            .join(presence(
                Uuid::new_v4(),
                recipient_session_id,
                server_id,
                room_id,
                Uuid::new_v4(),
            ))
            .await;
        store
            .join(presence(
                Uuid::new_v4(),
                other_room_session_id,
                server_id,
                other_room_id,
                Uuid::new_v4(),
            ))
            .await;

        let recipients = store
            .media_recipient_sessions(&room_id, &sender_session_id)
            .await;

        assert_eq!(recipients, vec![recipient_session_id]);
    }

    #[tokio::test]
    async fn replacing_user_presence_makes_old_session_stale() {
        let store = InMemoryVoicePresenceStore::default();
        let server_id = Uuid::new_v4();
        let first_room_id = Uuid::new_v4();
        let second_room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let old_session_id = Uuid::new_v4();
        let new_session_id = Uuid::new_v4();

        store
            .join(presence(
                Uuid::new_v4(),
                old_session_id,
                server_id,
                first_room_id,
                user_id,
            ))
            .await;
        store
            .join(presence(
                Uuid::new_v4(),
                new_session_id,
                server_id,
                second_room_id,
                user_id,
            ))
            .await;

        assert!(
            store
                .room_presence_for_user(&first_room_id, &user_id)
                .await
                .is_none()
        );
        assert_eq!(
            store
                .room_presence_for_user(&second_room_id, &user_id)
                .await
                .expect("new presence should remain")
                .session_id,
            new_session_id
        );
    }
}
