//! Атомарный вход голосового устройства с проверкой подтверждения замены.

use std::collections::HashSet;
use std::sync::Arc;

use super::{PresenceRegistry, RoomPublication, join_publication_plan};
use crate::features::voice_chat::infrastructure::{
    VoiceJoinConflict, VoicePresence, VoicePresenceTargetKind,
};

impl PresenceRegistry {
    /// Совместимость со старым вызывающим кодом до перевода приложения на подтверждения.
    /// Прежний вход вытесняет все подключения аккаунта.
    #[cfg(test)]
    pub(crate) async fn join(&self, presence: VoicePresence) -> Vec<VoicePresence> {
        self.join_inner(presence, true, true)
            .await
            .expect("legacy forced voice join cannot require confirmation")
    }

    /// Проверяет конфликт и изменяет присутствие под одной блокировкой.
    pub(crate) async fn try_join(
        &self,
        presence: VoicePresence,
        confirm_disconnect_other_devices: bool,
    ) -> Result<Vec<VoicePresence>, VoiceJoinConflict> {
        self.join_inner(presence, confirm_disconnect_other_devices, false)
            .await
    }

    async fn join_inner(
        &self,
        presence: VoicePresence,
        confirmed: bool,
        legacy_replace_all: bool,
    ) -> Result<Vec<VoicePresence>, VoiceJoinConflict> {
        let mut state = self.state.lock().await;
        let mut previous = state
            .streams_by_user
            .get(&presence.user_id)
            .into_iter()
            .flatten()
            .filter_map(|stream_id| state.by_stream.get(stream_id))
            .cloned()
            .collect::<Vec<_>>();
        if let Some(stream_presence) = state.by_stream.get(&presence.realtime_stream_id).cloned()
            && !previous
                .iter()
                .any(|entry| entry.realtime_stream_id == stream_presence.realtime_stream_id)
        {
            previous.push(stream_presence);
        }

        let other_connections = previous
            .iter()
            .filter(|old| old.realtime_stream_id != presence.realtime_stream_id)
            .collect::<Vec<_>>();
        let same_auth_conflict = other_connections
            .iter()
            .any(|old| old.auth_session_id == presence.auth_session_id);
        let target_conflict = other_connections.iter().any(|old| {
            old.target() != presence.target()
                || (presence.target_kind == VoicePresenceTargetKind::DirectMessage
                    && old.auth_session_id != presence.auth_session_id)
        });

        // До v5 не существовало диалога подтверждения. Повторный вход старой
        // версии в ту же комнату заменяет её прежнюю вкладку, но не вытесняет
        // v5-подключение и не разрешает переход в другую комнату.
        let legacy_same_room_reconnect = presence.voice_protocol_version < 5
            && same_auth_conflict
            && other_connections
                .iter()
                .filter(|old| old.auth_session_id == presence.auth_session_id)
                .all(|old| old.voice_protocol_version < 5 && old.target() == presence.target());
        if !confirmed && !legacy_replace_all {
            if same_auth_conflict && !legacy_same_room_reconnect {
                return Err(VoiceJoinConflict::SameAuthSessionConfirmationRequired);
            }
            if target_conflict {
                return Err(VoiceJoinConflict::DeviceTransitionConfirmationRequired);
            }
        }
        if legacy_same_room_reconnect {
            tracing::info!(
                user_id = %presence.user_id,
                room_id = %presence.room_id,
                "legacy voice client replaced its previous connection in the same room"
            );
        }

        // Устройствам разных auth-сессий разрешён совместный вход только
        // в одну серверную комнату. Все остальные переходы требуют handoff.
        let replace_all = legacy_replace_all || target_conflict;
        let to_remove = previous
            .into_iter()
            .filter(|old| {
                replace_all
                    || old.realtime_stream_id == presence.realtime_stream_id
                    || old.auth_session_id == presence.auth_session_id
            })
            .collect::<Vec<_>>();
        let mut affected_rooms = HashSet::new();
        let mut removed = Vec::with_capacity(to_remove.len());
        for old in &to_remove {
            // Закрываем прежнюю авторизацию потока до публикации нового маршрута.
            self.index.remove_stream(&old.realtime_stream_id);
        }
        for old in &to_remove {
            self.remove_from_state(&mut state, old, &mut affected_rooms);
            removed.push((**old).clone());
        }

        let presence = Arc::new(presence);
        self.insert_into_state(&mut state, Arc::clone(&presence));
        for publication in join_publication_plan(affected_rooms, presence.target()) {
            match publication {
                RoomPublication::Removal(target) | RoomPublication::Addition(target) => {
                    self.publish_room(&state, target);
                }
            }
        }
        self.index.insert_stream(presence);
        removed.sort_by_key(|old| old.joined_at);
        Ok(removed)
    }
}
