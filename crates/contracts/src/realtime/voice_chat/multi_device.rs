//! Контракты отдельных голосовых подключений и подтверждённого переключения устройств.

use serde::{Deserialize, Serialize};

use super::VoiceNetworkTargetKind;

/// Тип устройства для отображения отдельного голосового подключения.
///
/// Метка предоставляется клиентом и не используется для авторизации.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceDeviceKind {
    /// Настольный клиент.
    Desktop,
    /// Мобильный клиент.
    Mobile,
    /// Планшетный клиент.
    Tablet,
    /// Клиент в браузере.
    Web,
    /// Тип устройства не указан.
    #[default]
    Unknown,
}

/// Причина адресного вытеснения одного голосового подключения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoicePresenceEvictionReason {
    /// Подтверждён переход в другую голосовую цель с другого устройства аккаунта.
    AnotherDeviceTransition,
    /// Другое подключение с той же авторизацией заняло единственный голосовой слот.
    ///
    /// Клиент показывает предупреждение о потенциальном постороннем использовании сеанса.
    SameAuthSessionTakeover,
}

/// Адресное уведомление о вытеснении конкретного голосового присутствия.
///
/// Обычная авторизация и текстовые realtime-подключения этим событием не закрываются.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoicePresenceEvicted {
    /// UUID вытесненного основного WebTransport-подключения.
    pub session_id: String,
    /// Момент создания вытесненного присутствия: отличает повторные входы одного транспорта.
    #[serde(default)]
    pub joined_at: String,
    /// Вид голосовой цели.
    pub target_kind: VoiceNetworkTargetKind,
    /// Маршрутный идентификатор сервера или личного диалога.
    pub server_id: String,
    /// Идентификатор комнаты или личного диалога.
    pub room_id: String,
    /// Причина отключения, определяющая пользовательское предупреждение.
    pub reason: VoicePresenceEvictionReason,
}
