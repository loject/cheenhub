//! Общие контракты realtime-конвертов.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;

use super::control::ControlKind;
use super::network::NetworkKind;
use super::server::ServerKind;
use super::text_chat::TextChatKind;
use super::voice_chat::VoiceChatKind;

/// Верхнеуровневое пространство имен модулей realtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeModule {
    /// Сообщения жизненного цикла сессии и диагностического управления.
    Control,
    /// Сообщения измерения качества соединения.
    Network,
    /// Сообщения управления сервером и события.
    Server,
    /// Сообщения текстового чата и события.
    TextChat,
    /// Сообщения присутствия в голосовом чате и события.
    VoiceChat,
}

/// Обертка для типизированных видов сообщений realtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RealtimeKind {
    /// Вид сообщения модуля управления.
    Control(ControlKind),
    /// Вид сообщения модуля сети.
    Network(NetworkKind),
    /// Вид сообщения модуля управления сервером.
    Server(ServerKind),
    /// Вид сообщения модуля текстового чата.
    TextChat(TextChatKind),
    /// Вид сообщения модуля присутствия в голосовом чате.
    VoiceChat(VoiceChatKind),
}

impl RealtimeKind {
    /// Возвращает модуль, которому принадлежит этот вид.
    pub fn module(self) -> RealtimeModule {
        match self {
            Self::Control(_) => RealtimeModule::Control,
            Self::Network(_) => RealtimeModule::Network,
            Self::Server(_) => RealtimeModule::Server,
            Self::TextChat(_) => RealtimeModule::TextChat,
            Self::VoiceChat(_) => RealtimeModule::VoiceChat,
        }
    }
}

impl Serialize for RealtimeKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Control(kind) => kind.serialize(serializer),
            Self::Network(kind) => kind.serialize(serializer),
            Self::Server(kind) => kind.serialize(serializer),
            Self::TextChat(kind) => kind.serialize(serializer),
            Self::VoiceChat(kind) => kind.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for RealtimeKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        if let Ok(kind) = ControlKind::deserialize(value.clone()) {
            return Ok(Self::Control(kind));
        }
        if let Ok(kind) = NetworkKind::deserialize(value.clone()) {
            return Ok(Self::Network(kind));
        }
        if let Ok(kind) = ServerKind::deserialize(value.clone()) {
            return Ok(Self::Server(kind));
        }
        if let Ok(kind) = TextChatKind::deserialize(value.clone()) {
            return Ok(Self::TextChat(kind));
        }
        if let Ok(kind) = VoiceChatKind::deserialize(value) {
            return Ok(Self::VoiceChat(kind));
        }

        Err(serde::de::Error::custom("unknown realtime kind"))
    }
}

/// Конверт, используемый в каждом надежном сообщении realtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealtimeEnvelope {
    /// Модуль, владеющий сообщением.
    pub module: RealtimeModule,
    /// Локальный для модуля типизированный вид сообщения.
    pub kind: RealtimeKind,
    /// Необязательный идентификатор запроса, используемый для корреляции запрос-ответ.
    pub request_id: Option<Uuid>,
    /// JSON-полезная нагрузка, принадлежащая модулю и декодируемая принимающим модулем.
    pub payload: Value,
}

impl RealtimeEnvelope {
    /// Создает типизированный конверт из сериализуемой полезной нагрузки.
    pub fn new<T>(
        module: RealtimeModule,
        kind: RealtimeKind,
        request_id: Option<Uuid>,
        payload: T,
    ) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        Ok(Self {
            module,
            kind,
            request_id,
            payload: serde_json::to_value(payload)?,
        })
    }

    /// Возвращает, соответствует ли вид сообщения модулю конверта.
    pub fn has_matching_module_kind(&self) -> bool {
        self.kind.module() == self.module
    }
}
