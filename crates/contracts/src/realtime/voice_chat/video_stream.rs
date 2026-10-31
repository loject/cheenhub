//! Контракты локального видеопотока и его завершения в голосовой комнате.

use serde::{Deserialize, Serialize};

/// Источник видеопотока голосовой комнаты.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceVideoStreamSource {
    /// Видео с камеры участника.
    Camera,
    /// Демонстрация экрана участника.
    ScreenShare,
}

/// Полезная нагрузка сообщения об остановке локального видеопотока.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopVoiceVideoStream {
    /// UUID системного аудио завершённого захвата; `None` не останавливает аудиодорожки.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_track_id: Option<uuid::Uuid>,
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Остановленный источник видео.
    pub source: VoiceVideoStreamSource,
}

/// Событие остановки видеопотока участника голосовой комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceVideoStreamEnded {
    /// UUID подключения; отсутствие поля означает старый сервер.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session_id: String,
    /// UUID остановленного системного аудио; отсутствие поля сохраняет legacy-семантику видео.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_track_id: Option<uuid::Uuid>,
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Пользователь, остановивший видеопоток.
    pub user_id: String,
    /// Остановленный источник видео.
    pub source: VoiceVideoStreamSource,
}
