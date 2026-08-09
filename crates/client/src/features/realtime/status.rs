//! Состояние realtime-соединения.

/// Локальная причина перехода с WebTransport на резервный транспорт.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WebTransportFallbackReason {
    /// Попытка подключения не завершилась за клиентский таймаут.
    Timeout,
    /// Не удалось разрешить имя realtime-сервера.
    Dns,
    /// Не удалось установить или проверить TLS-соединение.
    Tls,
    /// WebTransport подключился, но realtime-аутентификация не завершилась успешно.
    Authentication,
    /// Произошла иная известная транспортная ошибка.
    Transport,
    /// Клиент не смог надёжно классифицировать причину.
    Unknown,
}

/// Диагностика причины активного резервного realtime-соединения.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RealtimeFallbackInfo {
    /// Нормализованная причина сбоя WebTransport.
    pub(crate) reason: WebTransportFallbackReason,
    /// Длительность неуспешной попытки WebTransport в миллисекундах.
    pub(crate) webtransport_elapsed_ms: u64,
}

impl RealtimeFallbackInfo {
    /// Возвращает стабильный код диагностики для интерфейса и поддержки.
    pub(crate) fn diagnostic_code(self) -> &'static str {
        match self.reason {
            WebTransportFallbackReason::Timeout => "RT-WT-TIMEOUT",
            WebTransportFallbackReason::Dns => "RT-WT-DNS",
            WebTransportFallbackReason::Tls => "RT-WT-TLS",
            WebTransportFallbackReason::Authentication => "RT-WT-AUTH",
            WebTransportFallbackReason::Transport => "RT-WT-TRANSPORT",
            WebTransportFallbackReason::Unknown => "RT-WT-UNKNOWN",
        }
    }
}

/// Активный тип realtime-транспорта.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RealtimeTransportKind {
    /// Primary WebTransport connection.
    WebTransport,
    /// Slower WebSocket fallback connection.
    WebSocketFallback,
}

/// Текущее состояние realtime-соединения.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RealtimeConnectionStatus {
    /// Выполняется попытка подключения через WebTransport.
    ConnectingWebTransport,
    /// Выполняется попытка резервного подключения через WebSocket.
    ConnectingWebSocketFallback,
    /// Realtime session is authenticated and ready for requests.
    Connected(RealtimeTransportKind),
    /// Realtime session is not ready.
    Disconnected,
}
