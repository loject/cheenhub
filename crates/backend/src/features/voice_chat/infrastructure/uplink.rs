//! In-memory разрешения для отдельной сессии отправки микрофона.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{InMemoryVoicePresenceStore, VoicePresence};

/// Одноразовое разрешение на привязку отдельной сессии отправки микрофона.
#[derive(Debug, Clone)]
pub(crate) struct MicrophoneUplinkGrant {
    /// Непредсказуемый идентификатор разрешения.
    pub(crate) id: Uuid,
    /// Пользователь, которому выдано разрешение.
    pub(crate) user_id: Uuid,
    /// Комната, для которой выдано разрешение.
    pub(crate) room_id: Uuid,
    /// Основная сессия, владеющая активным присутствием.
    pub(crate) presence_session_id: Uuid,
    /// Момент истечения разрешения.
    pub(crate) expires_at: DateTime<Utc>,
}

/// Активная привязка отдельной сессии отправки микрофона.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MicrophoneUplinkBinding {
    /// Привязанная realtime-сессия worker.
    pub(crate) session_id: Uuid,
    /// Пользователь привязанной сессии.
    pub(crate) user_id: Uuid,
    /// Комната, в которую разрешена отправка.
    pub(crate) room_id: Uuid,
    /// Основная сессия, владеющая активным присутствием.
    pub(crate) presence_session_id: Uuid,
}

/// Причина отказа при потреблении grant отправки микрофона.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsumeMicrophoneUplinkGrantError {
    /// Grant отсутствует или принадлежит другому пользователю.
    Invalid,
    /// Срок действия grant истек.
    Expired,
}

impl InMemoryVoicePresenceStore {
    /// Сохраняет короткоживущий одноразовый grant отправки микрофона.
    pub(crate) async fn issue_microphone_uplink_grant(&self, grant: MicrophoneUplinkGrant) {
        self.microphone_uplink_bindings
            .lock()
            .await
            .retain(|entry| {
                entry.user_id != grant.user_id
                    || entry.room_id != grant.room_id
                    || entry.presence_session_id != grant.presence_session_id
            });
        let mut grants = self.microphone_uplink_grants.lock().await;
        grants.retain(|entry| {
            entry.expires_at > Utc::now()
                && !(entry.user_id == grant.user_id && entry.room_id == grant.room_id)
        });
        grants.push(grant);
    }

    /// Однократно потребляет grant и привязывает текущую worker-сессию.
    pub(crate) async fn consume_microphone_uplink_grant(
        &self,
        grant_id: &Uuid,
        user_id: &Uuid,
        session_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<MicrophoneUplinkBinding, ConsumeMicrophoneUplinkGrantError> {
        let grant = {
            let mut grants = self.microphone_uplink_grants.lock().await;
            let Some(index) = grants
                .iter()
                .position(|entry| &entry.id == grant_id && &entry.user_id == user_id)
            else {
                return Err(ConsumeMicrophoneUplinkGrantError::Invalid);
            };
            let grant = grants.remove(index);
            if grant.expires_at <= now {
                return Err(ConsumeMicrophoneUplinkGrantError::Expired);
            }
            grant
        };
        let binding = MicrophoneUplinkBinding {
            session_id,
            user_id: *user_id,
            room_id: grant.room_id,
            presence_session_id: grant.presence_session_id,
        };
        let mut bindings = self.microphone_uplink_bindings.lock().await;
        bindings.retain(|entry| entry.session_id != session_id);
        bindings.push(binding.clone());
        Ok(binding)
    }

    /// Проверяет привязку worker-сессии к текущему активному присутствию.
    pub(crate) async fn microphone_uplink_is_bound(
        &self,
        session_id: &Uuid,
        user_id: &Uuid,
        room_id: &Uuid,
        presence_session_id: &Uuid,
    ) -> bool {
        self.microphone_uplink_bindings
            .lock()
            .await
            .iter()
            .any(|entry| {
                &entry.session_id == session_id
                    && &entry.user_id == user_id
                    && &entry.room_id == room_id
                    && &entry.presence_session_id == presence_session_id
            })
    }

    pub(super) async fn revoke_microphone_uplinks_for(&self, presences: &[VoicePresence]) {
        if presences.is_empty() {
            return;
        }
        let session_ids = presences
            .iter()
            .map(|presence| presence.session_id)
            .collect::<Vec<_>>();
        self.microphone_uplink_grants
            .lock()
            .await
            .retain(|grant| !session_ids.contains(&grant.presence_session_id));
        self.microphone_uplink_bindings
            .lock()
            .await
            .retain(|binding| !session_ids.contains(&binding.presence_session_id));
    }
}

#[cfg(test)]
mod tests;
