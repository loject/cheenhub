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
    /// Присоединиться к голосовому звонку личного диалога.
    JoinDirectMessageVoiceRoom,
    /// Покинуть голосовой звонок личного диалога.
    LeaveDirectMessageVoiceRoom,
    /// Исключить одного участника из голосовой комнаты.
    KickVoiceMember,
    /// Загрузить снимки присутствия участников в активных голосовых комнатах для одного сервера.
    ListServerVoiceRooms,
    /// Загрузить активные голосовые звонки личных диалогов пользователя.
    ListDirectMessageVoiceRooms,
    /// Сообщить об остановке локального видеопотока в голосовой комнате.
    StopVideoStream,
    /// Выдать одноразовый grant для отдельной сессии отправки микрофона.
    IssueMicrophoneUplinkGrant,
    /// Одноразовый grant для отдельной сессии отправки микрофона выдан.
    MicrophoneUplinkGrantIssued,
    /// Привязать текущую realtime-сессию к отправке микрофона по grant.
    BindMicrophoneUplink,
    /// Текущая realtime-сессия привязана к отправке микрофона.
    MicrophoneUplinkBound,
    /// Снимки активных голосовых комнат для одного сервера.
    ServerVoiceRoomsSnapshot,
    /// Снимки активных голосовых звонков личных диалогов.
    DirectMessageVoiceRoomsSnapshot,
    /// Текущий снимок участников голосовой комнаты.
    VoiceRoomSnapshot,
    /// Событие изменения списка участников голосовой комнаты.
    ParticipantsChanged,
    /// Событие остановки видеопотока участника голосовой комнаты.
    VideoStreamEnded,
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

/// Запрос одноразового grant для отдельной сессии отправки микрофона.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueMicrophoneUplinkGrant {
    /// Идентификатор комнаты, в которой основная сессия уже присутствует.
    pub room_id: String,
}

/// Выданный одноразовый grant для отдельной сессии отправки микрофона.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicrophoneUplinkGrantIssued {
    /// Непредсказуемый UUID grant, который должен предъявить worker.
    pub grant: String,
    /// Идентификатор комнаты, к которой относится grant.
    pub room_id: String,
    /// Момент истечения grant в формате RFC3339.
    pub expires_at: String,
}

/// Запрос привязки текущей realtime-сессии к отправке микрофона.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindMicrophoneUplink {
    /// Одноразовый UUID grant, полученный основной realtime-сессией.
    pub grant: String,
}

/// Подтверждение привязки realtime-сессии к отправке микрофона.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicrophoneUplinkBound {
    /// Идентификатор комнаты, для которой разрешена отправка микрофона.
    pub room_id: String,
}

/// Полезная нагрузка запроса на присоединение к голосовому звонку личного диалога.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinDirectMessageVoiceRoom {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
}

/// Полезная нагрузка запроса на выход из голосового звонка личного диалога.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveDirectMessageVoiceRoom {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
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

/// Полезная нагрузка запроса на загрузку активных голосовых звонков личных диалогов.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDirectMessageVoiceRooms;

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
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Пользователь, остановивший видеопоток.
    pub user_id: String,
    /// Остановленный источник видео.
    pub source: VoiceVideoStreamSource,
}

/// Снимки активных голосовых комнат одного сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerVoiceRoomsSnapshot {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Снимки голосовых комнат с активными участниками.
    pub rooms: Vec<VoiceRoomSnapshot>,
}

/// Снимки активных голосовых звонков личных диалогов.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageVoiceRoomsSnapshot {
    /// Снимки голосовых звонков личных диалогов с активными участниками.
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
