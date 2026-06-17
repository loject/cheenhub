//! Контракты realtime-модуля качества сети.

use serde::{Deserialize, Serialize};

/// Виды сообщений сетевого модуля.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkKind {
    /// Надежный ping-запрос.
    Ping,
    /// Надежный pong-ответ.
    Pong,
}

/// Полезная нагрузка надежного ping-запроса.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    /// Временная метка на стороне клиента в миллисекундах.
    pub sent_at_ms: u64,
}

/// Полезная нагрузка надежного pong-ответа.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    /// Исходная временная метка клиента в миллисекундах.
    pub sent_at_ms: u64,
    /// Временная метка получения на стороне сервера в миллисекундах.
    pub server_received_at_ms: u64,
    /// Временная метка отправки на стороне сервера в миллисекундах.
    pub server_sent_at_ms: u64,
}
