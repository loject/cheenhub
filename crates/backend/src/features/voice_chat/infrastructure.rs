//! Инфраструктура присутствия голосового чата.

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

/// In-memory-хранилище голосового присутствия для активных потоков realtime-модуля.
#[derive(Default)]
pub(crate) struct InMemoryVoicePresenceStore {
    entries: Mutex<Vec<VoicePresence>>,
}

/// Активная запись присутствия в голосовой комнате.
#[derive(Debug, Clone)]
pub(crate) struct VoicePresence {
    /// Поток realtime-модуля, которому принадлежит это присутствие и который используется для очистки при отключении.
    pub(crate) realtime_stream_id: Uuid,
    /// Аутентифицированная сессия WebTransport, получающая медиадатаграммы.
    pub(crate) session_id: Uuid,
    /// Тип цели голосового присутствия.
    pub(crate) target_kind: VoicePresenceTargetKind,
    /// Сервер, содержащий присоединенную комнату.
    pub(crate) server_id: Uuid,
    /// Идентификатор присоединенной комнаты.
    pub(crate) room_id: Uuid,
    /// Пользователь, вошедший в комнату.
    pub(crate) user_id: Uuid,
    /// Снимок ника пользователя.
    pub(crate) nickname: String,
    /// Снимок публичного URL аватара.
    pub(crate) avatar_url: Option<String>,
    /// Время присоединения.
    pub(crate) joined_at: DateTime<Utc>,
}

/// Тип цели голосового присутствия.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum VoicePresenceTargetKind {
    /// Серверная голосовая комната.
    Server,
    /// Голосовой звонок личного диалога.
    DirectMessage,
}

/// Ключ цели голосового присутствия.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VoicePresenceTarget {
    /// Тип цели.
    pub(crate) kind: VoicePresenceTargetKind,
    /// Маршрутный идентификатор цели.
    pub(crate) server_id: Uuid,
    /// Идентификатор комнаты или личного диалога.
    pub(crate) room_id: Uuid,
}

impl InMemoryVoicePresenceStore {
    /// Заменяет присутствие одного пользователя или realtime-потока и возвращает удаленные записи.
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

    /// Удаляет присутствие для одного потока realtime-модуля.
    pub(crate) async fn leave_realtime_stream(
        &self,
        realtime_stream_id: &Uuid,
    ) -> Vec<VoicePresence> {
        self.remove_presence(|entry| &entry.realtime_stream_id == realtime_stream_id)
            .await
    }

    /// Удаляет присутствие для одного потока realtime-модуля в одной комнате.
    pub(crate) async fn leave_room(
        &self,
        realtime_stream_id: &Uuid,
        target_kind: VoicePresenceTargetKind,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        self.remove_presence(|entry| {
            &entry.realtime_stream_id == realtime_stream_id
                && entry.target_kind == target_kind
                && &entry.server_id == server_id
                && &entry.room_id == room_id
        })
        .await
    }

    /// Удаляет все записи присутствия одного пользователя в одной комнате (kick).
    pub(crate) async fn kick_user_from_room(
        &self,
        user_id: &Uuid,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        self.remove_presence(|entry| {
            entry.target_kind == VoicePresenceTargetKind::Server
                && &entry.user_id == user_id
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

    /// Перечисляет активных участников одной комнаты.
    pub(crate) async fn room_participants(
        &self,
        target_kind: VoicePresenceTargetKind,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        let mut participants = self
            .entries
            .lock()
            .await
            .iter()
            .filter(|entry| {
                entry.target_kind == target_kind
                    && &entry.server_id == server_id
                    && &entry.room_id == room_id
            })
            .cloned()
            .collect::<Vec<_>>();
        participants.sort_by_key(|presence| presence.joined_at);
        participants
    }

    /// Перечисляет активных участников одного сервера, сгруппированных по комнатам.
    pub(crate) async fn server_room_participants(
        &self,
        server_id: &Uuid,
    ) -> Vec<(Uuid, Vec<VoicePresence>)> {
        let mut entries = self
            .entries
            .lock()
            .await
            .iter()
            .filter(|entry| {
                entry.target_kind == VoicePresenceTargetKind::Server
                    && &entry.server_id == server_id
            })
            .cloned()
            .collect::<Vec<_>>();
        entries.sort_by_key(|presence| (presence.room_id, presence.joined_at));

        let mut rooms = Vec::<(Uuid, Vec<VoicePresence>)>::new();
        for presence in entries {
            match rooms.last_mut() {
                Some((room_id, participants)) if *room_id == presence.room_id => {
                    participants.push(presence);
                }
                _ => rooms.push((presence.room_id, vec![presence])),
            }
        }

        rooms
    }

    /// Возвращает активное присутствие одного пользователя в одной комнате.
    pub(crate) async fn room_presence_for_user(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<VoicePresence> {
        self.entries
            .lock()
            .await
            .iter()
            .find(|entry| {
                entry.target_kind == target_kind
                    && &entry.room_id == room_id
                    && &entry.user_id == user_id
            })
            .cloned()
    }

    /// Обновляет никнейм в активных записях присутствия одного пользователя и возвращает затронутые идентификаторы комнат.
    pub(crate) async fn update_user_nickname(
        &self,
        user_id: &Uuid,
        nickname: String,
    ) -> Vec<VoicePresenceTarget> {
        let mut entries = self.entries.lock().await;
        let mut rooms = Vec::<VoicePresenceTarget>::new();

        for entry in entries.iter_mut().filter(|entry| &entry.user_id == user_id) {
            entry.nickname = nickname.clone();
            let room = entry.target();
            if !rooms.contains(&room) {
                rooms.push(room);
            }
        }

        rooms
    }

    /// Обновляет URL аватара в активных записях присутствия одного пользователя и возвращает затронутые идентификаторы комнат.
    pub(crate) async fn update_user_avatar(
        &self,
        user_id: &Uuid,
        avatar_url: Option<String>,
    ) -> Vec<VoicePresenceTarget> {
        let mut entries = self.entries.lock().await;
        let mut rooms = Vec::<VoicePresenceTarget>::new();

        for entry in entries.iter_mut().filter(|entry| &entry.user_id == user_id) {
            entry.avatar_url = avatar_url.clone();
            let room = entry.target();
            if !rooms.contains(&room) {
                rooms.push(room);
            }
        }

        rooms
    }

    /// Перечисляет активных получателей медиа в одной комнате, исключая одну сессию отправителя.
    pub(crate) async fn media_recipient_sessions(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        sender_session_id: &Uuid,
    ) -> Vec<Uuid> {
        self.entries
            .lock()
            .await
            .iter()
            .filter(|entry| {
                entry.target_kind == target_kind
                    && &entry.room_id == room_id
                    && &entry.session_id != sender_session_id
            })
            .map(|entry| entry.session_id)
            .collect()
    }
}

impl VoicePresence {
    /// Возвращает ключ цели присутствия.
    pub(crate) fn target(&self) -> VoicePresenceTarget {
        VoicePresenceTarget {
            kind: self.target_kind,
            server_id: self.server_id,
            room_id: self.room_id,
        }
    }
}

impl VoicePresenceTarget {
    /// Возвращает маршрутный идентификатор для payload'ов старого формата.
    pub(crate) fn route_id(self) -> Uuid {
        self.server_id
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::{InMemoryVoicePresenceStore, VoicePresence, VoicePresenceTargetKind};

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
            target_kind: VoicePresenceTargetKind::Server,
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
                .room_presence_for_user(VoicePresenceTargetKind::Server, &room_id, &user_id)
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
                .room_presence_for_user(VoicePresenceTargetKind::Server, &room_id, &user_id)
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
            .media_recipient_sessions(
                VoicePresenceTargetKind::Server,
                &room_id,
                &sender_session_id,
            )
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
                .room_presence_for_user(VoicePresenceTargetKind::Server, &first_room_id, &user_id)
                .await
                .is_none()
        );
        assert_eq!(
            store
                .room_presence_for_user(VoicePresenceTargetKind::Server, &second_room_id, &user_id)
                .await
                .expect("new presence should remain")
                .session_id,
            new_session_id
        );
    }
}
