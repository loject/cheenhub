//! Единая граница владения аутентифицированными realtime-сессиями.
//!
//! Все добавления и удаления проходят через этот реестр, поэтому счётчики
//! присутствия обновляются вместе с действительными записями соединений.

use dashmap::DashMap;
use tokio::sync::{Mutex, broadcast, watch};
use tracing::{debug, info};
use uuid::Uuid;

use super::disconnect::DisconnectReason;
use super::session_activity::{SessionActivity, SessionActivityChange, SessionActivityRegistry};
use crate::realtime::sink::DatagramSink;

/// Одна аутентифицированная транспортная сессия для датаграмм и отключения.
#[derive(Clone)]
pub(super) struct RealtimeSession {
    /// Идентификатор транспортного подключения.
    pub(super) id: Uuid,
    /// Аутентифицированный владелец подключения.
    pub(super) user_id: Uuid,
    /// Auth-сессия, отзыв которой закрывает этот транспорт.
    pub(super) auth_session_id: Uuid,
    /// Отправитель ненадёжных датаграмм.
    pub(super) datagrams: DatagramSink,
    /// Сигнал завершения с причиной.
    pub(super) disconnect: watch::Sender<Option<DisconnectReason>>,
}

/// Реестр транспортов и производного присутствия пользователей.
///
/// Мьютекс сериализует изменение записей сессий и обновление счётчиков,
/// а DashMap сохраняет шардированный доступ в горячем пути медиадоставки.
#[derive(Default)]
pub(super) struct SessionRegistry {
    /// Зарегистрированные транспорты по идентификатору сессии.
    entries: DashMap<Uuid, RealtimeSession>,
    activity: Mutex<SessionActivityRegistry>,
}

impl SessionRegistry {
    /// Регистрирует транспорт один раз и возвращает подписку на его закрытие.
    pub(super) async fn register_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        auth_session_id: Uuid,
        datagrams: DatagramSink,
    ) -> watch::Receiver<Option<DisconnectReason>> {
        let mut activity = self.activity.lock().await;
        let receiver = match self.entries.entry(session_id) {
            dashmap::Entry::Occupied(session) => session.get().disconnect.subscribe(),
            dashmap::Entry::Vacant(session) => {
                let (disconnect, receiver) = watch::channel(None);
                session.insert(RealtimeSession {
                    id: session_id,
                    user_id,
                    auth_session_id,
                    datagrams,
                    disconnect,
                });
                activity.connected(user_id);
                receiver
            }
        };
        debug!(%session_id, %user_id, %auth_session_id, "registered realtime session");
        receiver
    }

    /// Удаляет транспорт, если он ещё зарегистрирован.
    pub(super) async fn unregister_session(&self, session_id: Uuid) {
        let mut activity = self.activity.lock().await;
        if let Some((_, session)) = self.entries.remove(&session_id) {
            activity.disconnected(session.user_id);
        }
        debug!(%session_id, "unregistered realtime session");
    }

    /// Снимает транспортный снимок для медиадоставки без удержания guard.
    pub(super) fn get(&self, session_id: &Uuid) -> Option<RealtimeSession> {
        self.entries
            .get(session_id)
            .map(|entry| entry.value().clone())
    }

    /// Закрывает все транспорты одной auth-сессии.
    pub(super) async fn disconnect_auth_session(&self, auth_session_id: &Uuid) -> usize {
        self.disconnect_matching(
            |session| session.auth_session_id == *auth_session_id,
            DisconnectReason::AuthSessionRevoked,
            "revoked auth session",
        )
        .await
    }

    /// Закрывает все транспорты пользователя.
    pub(super) async fn disconnect_user_sessions(&self, user_id: &Uuid) -> usize {
        self.disconnect_matching(
            |session| session.user_id == *user_id,
            DisconnectReason::AuthSessionRevoked,
            "revoked user sessions",
        )
        .await
    }

    /// Закрывает все транспорты перед остановкой процесса.
    pub(super) async fn disconnect_all_sessions(&self) -> usize {
        self.disconnect_matching(
            |_| true,
            DisconnectReason::ServiceRestarting,
            "process exit",
        )
        .await
    }

    /// Возвращает получателя событий изменения активности.
    pub(super) async fn subscribe_activity(&self) -> broadcast::Receiver<SessionActivityChange> {
        self.activity.lock().await.subscribe()
    }

    /// Возвращает снимок активности без проверки права доступа к нему.
    pub(super) async fn activity_for_user(&self, user_id: Uuid) -> SessionActivity {
        self.activity.lock().await.snapshot(user_id)
    }

    /// Удаляет подходящие транспорты и обновляет присутствие одной операцией.
    async fn disconnect_matching(
        &self,
        matches: impl Fn(&RealtimeSession) -> bool,
        reason: DisconnectReason,
        context: &'static str,
    ) -> usize {
        let mut activity = self.activity.lock().await;
        let mut disconnected = 0;
        self.entries.retain(|_, session| {
            if !matches(session) {
                return true;
            }

            disconnected += 1;
            activity.disconnected(session.user_id);
            let _ = session.disconnect.send(Some(reason));
            false
        });
        let remaining = self.entries.len();
        if disconnected > 0 {
            info!(
                %reason,
                realtime_session_count = disconnected,
                "disconnected realtime transports"
            );
        } else {
            debug!(%reason, "no realtime transports matched disconnect request");
        }
        debug!(remaining_sessions = remaining, removed_sessions = disconnected, %context, "realtime session registry updated after disconnect request");
        disconnected
    }
}

#[cfg(test)]
mod tests {
    use dashmap::DashMap;
    use uuid::Uuid;

    use super::{RealtimeSession, SessionRegistry};

    #[test]
    fn session_registry_is_sharded_by_session_id() {
        fn assert_sharded_registry(_: &DashMap<Uuid, RealtimeSession>) {}

        assert_sharded_registry(&SessionRegistry::default().entries);
    }
}
