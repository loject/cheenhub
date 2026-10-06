//! Инфраструктура присутствия голосового чата.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::media_policy::VideoPublicationTracker;
use network_quality_rate_limit::NetworkQualityRateLimiter;

mod direct_calls;
mod network_quality_rate_limit;
mod presence_index;
mod presence_registry;
mod uplink;

pub(crate) use direct_calls::{
    DirectCall, DirectCallStoreError, DirectCallTransition, InMemoryDirectCallStore,
};
pub(crate) use presence_index::MediaRouteSnapshot;
use presence_registry::PresenceRegistry;
pub(crate) use uplink::{
    ConsumeMicrophoneUplinkGrantError, MicrophoneUplinkBinding, MicrophoneUplinkGrant,
};

/// In-memory-хранилище голосового присутствия для активных потоков realtime-модуля.
#[derive(Default)]
pub(crate) struct InMemoryVoicePresenceStore {
    /// Канонические записи присутствия и их read indexes.
    presence: PresenceRegistry,
    /// Одноразовые разрешения на подключение microphone uplink.
    microphone_uplink_grants: Mutex<Vec<MicrophoneUplinkGrant>>,
    /// Активные привязки дополнительных microphone uplink-сессий.
    microphone_uplink_bindings: Mutex<HashMap<Uuid, MicrophoneUplinkBinding>>,
    /// Ограничитель частоты публикации network quality.
    network_quality_rate_limiter: Mutex<NetworkQualityRateLimiter>,
    #[cfg(test)]
    /// Счётчик вызовов перечисления участников для проверок локальности чтения.
    room_participants_calls: AtomicUsize,
    /// Состояние активных видеоисточников.
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
        let removed = self.presence.join(presence).await;
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
        self.remove_presence(
            self.presence
                .leave_realtime_stream(realtime_stream_id)
                .await,
        )
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
        self.remove_presence(
            self.presence
                .leave_room(realtime_stream_id, target_kind, server_id, room_id)
                .await,
        )
        .await
    }

    /// Удаляет голосовое присутствие только в комнатах указанного сервера.
    ///
    /// Личные звонки и присутствие на других серверах сохраняются; связанные
    /// uplink-разрешения и состояние видеопубликаций очищаются вместе с presence.
    pub(crate) async fn remove_server(&self, server_id: &Uuid) -> Vec<VoicePresence> {
        self.remove_presence(self.presence.remove_server(server_id).await)
            .await
    }

    /// Удаляет все записи присутствия одного пользователя в одной комнате (kick).
    pub(crate) async fn kick_user_from_room(
        &self,
        user_id: &Uuid,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        self.remove_presence(
            self.presence
                .kick_user_from_room(user_id, server_id, room_id)
                .await,
        )
        .await
    }

    async fn remove_presence(&self, removed: Vec<VoicePresence>) -> Vec<VoicePresence> {
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
        self.presence
            .room_participants(target_kind, server_id, room_id)
            .await
    }

    /// Перечисляет активных участников одного сервера, сгруппированных по комнатам.
    pub(crate) async fn server_room_participants(
        &self,
        server_id: &Uuid,
    ) -> Vec<(Uuid, Vec<VoicePresence>)> {
        self.presence.server_room_participants(server_id).await
    }

    /// Возвращает активное присутствие одного пользователя в одной комнате.
    pub(crate) fn room_presence_for_user(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<Arc<VoicePresence>> {
        self.presence
            .room_presence_for_user(target_kind, room_id, user_id)
    }

    /// Возвращает присутствие отправителя и получателей из одного снимка комнаты.
    pub(crate) fn media_route(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<MediaRouteSnapshot> {
        self.presence.media_route(target_kind, room_id, user_id)
    }

    /// Возвращает присутствие, принадлежащее указанному realtime-потоку и пользователю.
    pub(crate) fn presence_for_stream(
        &self,
        realtime_stream_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<Arc<VoicePresence>> {
        self.presence
            .presence_for_stream(realtime_stream_id, user_id)
    }

    /// Обновляет никнейм в активных записях присутствия одного пользователя и возвращает затронутые идентификаторы комнат.
    pub(crate) async fn update_user_nickname(
        &self,
        user_id: &Uuid,
        nickname: String,
    ) -> Vec<VoicePresenceTarget> {
        self.presence.update_user_nickname(user_id, nickname).await
    }

    /// Обновляет URL аватара в активных записях присутствия одного пользователя и возвращает затронутые идентификаторы комнат.
    pub(crate) async fn update_user_avatar(
        &self,
        user_id: &Uuid,
        avatar_url: Option<String>,
    ) -> Vec<VoicePresenceTarget> {
        self.presence.update_user_avatar(user_id, avatar_url).await
    }

    /// Возвращает число активных голосовых подключений без объединения по пользователю.
    ///
    /// Учитываются комнаты серверов и личные звонки. Отдельное подключение для
    /// отправки микрофона присутствием не является и в счётчик не попадает.
    pub(crate) async fn active_voice_connection_count(&self) -> usize {
        self.presence.active_voice_connection_count().await
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
