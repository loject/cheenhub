//! Инфраструктура присутствия голосового чата.

use chrono::{DateTime, Utc};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::media_policy::VideoPublicationTracker;
use network_quality_rate_limit::NetworkQualityRateLimiter;

mod direct_calls;
mod network_quality_rate_limit;
mod uplink;

pub(crate) use direct_calls::{
    DirectCall, DirectCallStoreError, DirectCallTransition, InMemoryDirectCallStore,
};
pub(crate) use uplink::{
    ConsumeMicrophoneUplinkGrantError, MicrophoneUplinkBinding, MicrophoneUplinkGrant,
};

/// In-memory-хранилище голосового присутствия для активных потоков realtime-модуля.
#[derive(Default)]
pub(crate) struct InMemoryVoicePresenceStore {
    entries: Mutex<Vec<VoicePresence>>,
    microphone_uplink_grants: Mutex<Vec<MicrophoneUplinkGrant>>,
    microphone_uplink_bindings: Mutex<Vec<MicrophoneUplinkBinding>>,
    network_quality_rate_limiter: Mutex<NetworkQualityRateLimiter>,
    #[cfg(test)]
    room_participants_calls: AtomicUsize,
    pub(super) video_publications: Mutex<VideoPublicationTracker>,
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
        let removed = {
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
        };
        self.revoke_microphone_uplinks_for(&removed).await;
        self.clear_network_quality_rate_limits_for(&removed).await;
        self.clear_video_publications_for(&removed).await;

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

    /// Удаляет голосовое присутствие только в комнатах указанного сервера.
    ///
    /// Личные звонки и присутствие на других серверах сохраняются; связанные
    /// uplink-разрешения и состояние видеопубликаций очищаются вместе с presence.
    pub(crate) async fn remove_server(&self, server_id: &Uuid) -> Vec<VoicePresence> {
        self.remove_presence(|entry| {
            entry.target_kind == VoicePresenceTargetKind::Server && entry.server_id == *server_id
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
        let removed = {
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
        };
        self.revoke_microphone_uplinks_for(&removed).await;
        self.clear_network_quality_rate_limits_for(&removed).await;
        self.clear_video_publications_for(&removed).await;

        removed
    }

    async fn clear_video_publications_for(&self, removed: &[VoicePresence]) {
        self.video_publications
            .lock()
            .await
            .remove_presences(removed);
    }

    /// Перечисляет активных участников одной комнаты.
    pub(crate) async fn room_participants(
        &self,
        target_kind: VoicePresenceTargetKind,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        #[cfg(test)]
        self.room_participants_calls.fetch_add(1, Ordering::Relaxed);
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

    /// Возвращает присутствие, принадлежащее указанному realtime-потоку и пользователю.
    pub(crate) async fn presence_for_stream(
        &self,
        realtime_stream_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<VoicePresence> {
        self.entries
            .lock()
            .await
            .iter()
            .find(|entry| {
                &entry.realtime_stream_id == realtime_stream_id && &entry.user_id == user_id
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

    /// Возвращает число активных голосовых подключений без объединения по пользователю.
    ///
    /// Учитываются комнаты серверов и личные звонки. Отдельное подключение для
    /// отправки микрофона присутствием не является и в счётчик не попадает.
    pub(crate) async fn active_voice_connection_count(&self) -> usize {
        self.entries.lock().await.len()
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
mod tests;
