//! Общий контракт присутствия пользователя для REST и realtime.

use serde::{Deserialize, Serialize};

/// Публичное присутствие пользователя.
///
/// Время последнего присутствия указывается только для отключённого пользователя.
/// Отсутствие времени означает, что сервер не располагает этими сведениями.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UserPresence {
    /// Пользователь имеет хотя бы одну активную realtime-сессию.
    Online,
    /// У пользователя нет активных realtime-сессий.
    Offline {
        /// Момент последнего отключения в UTC (RFC3339), если он известен.
        last_seen_at: Option<String>,
    },
}

impl Default for UserPresence {
    fn default() -> Self {
        Self::Offline { last_seen_at: None }
    }
}
