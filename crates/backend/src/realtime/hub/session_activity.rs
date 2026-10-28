//! Метаданные жизненного цикла аутентифицированных realtime-сессий.
//!
//! Модуль не определяет, кому показывать присутствие: это политика social.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::RealtimeHub;

/// Снимок активности транспортных сессий пользователя в текущем процессе.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionActivity {
    /// Существует хотя бы одно активное соединение пользователя.
    pub(crate) online: bool,
    /// Время последнего закрытия всех соединений, если оно известно процессу.
    pub(crate) last_seen_at: Option<DateTime<Utc>>,
}

/// Изменение активности после первого подключения или последнего отключения.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SessionActivityChange {
    /// Пользователь, у которого изменилась активность сессий.
    pub(crate) user_id: Uuid,
    /// Состояние после изменения.
    pub(crate) activity: SessionActivity,
}

#[derive(Default)]
struct UserSessions {
    count: usize,
    last_seen_at: Option<DateTime<Utc>>,
}

/// Учёт количества сессий и последних отключений без постоянного хранения.
#[derive(Default)]
pub(super) struct SessionActivityRegistry {
    users: HashMap<Uuid, UserSessions>,
    publisher: Option<broadcast::Sender<SessionActivityChange>>,
}

impl SessionActivityRegistry {
    pub(super) fn subscribe(&mut self) -> broadcast::Receiver<SessionActivityChange> {
        self.publisher
            .get_or_insert_with(|| broadcast::channel(256).0)
            .subscribe()
    }

    pub(super) fn snapshot(&self, user_id: Uuid) -> SessionActivity {
        match self.users.get(&user_id) {
            Some(sessions) => SessionActivity {
                online: sessions.count > 0,
                last_seen_at: sessions.last_seen_at,
            },
            None => SessionActivity {
                online: false,
                last_seen_at: None,
            },
        }
    }

    pub(super) fn connected(&mut self, user_id: Uuid) {
        let sessions = self.users.entry(user_id).or_default();
        sessions.count += 1;
        if sessions.count == 1 {
            tracing::debug!(%user_id, "first realtime session connected");
            self.publish(user_id);
        }
    }

    pub(super) fn disconnected(&mut self, user_id: Uuid) {
        let Some(sessions) = self.users.get_mut(&user_id) else {
            return;
        };
        if sessions.count == 0 {
            return;
        }
        sessions.count -= 1;
        if sessions.count == 0 {
            sessions.last_seen_at = Some(Utc::now());
            tracing::debug!(%user_id, "last realtime session disconnected");
            self.publish(user_id);
        }
    }

    fn publish(&self, user_id: Uuid) {
        if let Some(publisher) = &self.publisher {
            let _ = publisher.send(SessionActivityChange {
                user_id,
                activity: self.snapshot(user_id),
            });
        }
    }
}

impl RealtimeHub {
    /// Подписывает потребителя на изменения активности транспортных сессий.
    ///
    /// При переполнении очереди получатель должен обновить свой снимок.
    pub(crate) async fn subscribe_session_activity(
        &self,
    ) -> broadcast::Receiver<SessionActivityChange> {
        self.session_activity.lock().await.subscribe()
    }

    /// Возвращает активность сессий пользователя без проверки права её просмотра.
    pub(crate) async fn session_activity_for_user(&self, user_id: Uuid) -> SessionActivity {
        self.session_activity.lock().await.snapshot(user_id)
    }
}
