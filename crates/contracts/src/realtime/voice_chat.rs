//! Контракты модуля realtime для присутствия в голосовом чате.

use serde::{Deserialize, Serialize};

/// Виды сообщений модуля присутствия в голосовом чате.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceChatKind {
    /// Присоединиться к одной комнате с поддержкой голоса.
    JoinVoiceRoom,
    /// Покинуть одну комнату с поддержкой голоса.
    LeaveVoiceRoom,
    /// Исключить одного участника из голосовой комнаты.
    KickVoiceMember,
    /// Загрузить снимки присутствия участников в активных голосовых комнатах для одного сервера.
    ListServerVoiceRooms,
    /// Снимки активных голосовых комнат для одного сервера.
    ServerVoiceRoomsSnapshot,
    /// Текущий снимок участников голосовой комнаты.
    VoiceRoomSnapshot,
    /// Событие изменения списка участников голосовой комнаты.
    ParticipantsChanged,
}

/// Полезная нагрузка запроса на присоединение к комнате с поддержкой голоса.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinVoiceRoom {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
}

/// Полезная нагрузка запроса на выход из комнаты с поддержкой голоса.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveVoiceRoom {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
}

/// Текущие участники одной голосовой комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomSnapshot {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Участники, присутствующие в комнате.
    pub participants: Vec<VoiceRoomParticipant>,
}

/// Полезная нагрузка запроса на исключение участника из голосовой комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KickVoiceMember {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор исключаемого пользователя.
    pub user_id: String,
}

/// Полезная нагрузка запроса на загрузку активных голосовых комнат одного сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerVoiceRooms {
    /// Идентификатор сервера.
    pub server_id: String,
}

/// Снимки активных голосовых комнат одного сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerVoiceRoomsSnapshot {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Снимки голосовых комнат с активными участниками.
    pub rooms: Vec<VoiceRoomSnapshot>,
}

/// Полезная нагрузка участника голосовой комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomParticipant {
    /// Стабильный идентификатор пользователя.
    pub user_id: String,
    /// Снимок имени пользователя.
    pub nickname: String,
    /// Публичный URL изображения аватара, если он настроен.
    pub avatar_url: Option<String>,
    /// Метка времени RFC3339, когда этот участник присоединился.
    pub joined_at: String,
}
