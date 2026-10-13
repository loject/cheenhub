//! Причины закрытия realtime-транспортов.

use crate::realtime::SERVICE_RESTARTING_CLOSE_CODE;

/// Причина, по которой backend закрывает активный realtime-транспорт.
///
/// Причина определяет код закрытия, который видит клиент: отзыв auth-сессии —
/// это потеря доступа, а плановый рестарт — временная недоступность сервера,
/// после которой клиент должен переподключиться сразу.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisconnectReason {
    /// Связанная с транспортом auth-сессия была отозвана.
    AuthSessionRevoked,
    /// Процесс переходит в фазу завершения.
    ServiceRestarting,
}

impl std::fmt::Display for DisconnectReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::AuthSessionRevoked => "auth_session_revoked",
            Self::ServiceRestarting => "service_restarting",
        };
        formatter.write_str(label)
    }
}

impl DisconnectReason {
    /// Возвращает код закрытия транспорта для этой причины.
    ///
    /// Для отзыва сессии используется 4003 из диапазона WebTransport, для
    /// планового рестарта — 1013 (try again later) по RFC 6455.
    pub(crate) fn close_code(self) -> u32 {
        match self {
            Self::AuthSessionRevoked => 4003,
            Self::ServiceRestarting => SERVICE_RESTARTING_CLOSE_CODE,
        }
    }

    /// Возвращает текст причины, который получает клиент при закрытии.
    pub(crate) fn close_message(self) -> &'static str {
        match self {
            Self::AuthSessionRevoked => "auth session revoked",
            Self::ServiceRestarting => "backend is shutting down",
        }
    }
}
