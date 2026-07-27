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
    /// Начать личный звонок.
    StartDirectCall,
    /// Принять или отклонить входящий личный звонок.
    RespondDirectCall,
    /// Отменить исходящий личный звонок до ответа.
    CancelDirectCall,
    /// Завершить активный личный звонок.
    EndDirectCall,
    /// Загрузить актуальные личные звонки пользователя.
    ListDirectCalls,
    /// Снимок актуальных личных звонков пользователя.
    DirectCallsSnapshot,
    /// Адресное событие изменения состояния личного звонка.
    DirectCallLifecycleEvent,
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

/// Запрос на начало личного звонка.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartDirectCall {
    /// Идентификатор личного диалога с вызываемым пользователем.
    pub conversation_id: String,
}

/// Ответ пользователя на входящий личный звонок.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectCallResponse {
    /// Принять звонок.
    Accept,
    /// Отклонить звонок.
    Decline,
}

/// Запрос на принятие или отклонение входящего личного звонка.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RespondDirectCall {
    /// Идентификатор звонка.
    pub call_id: String,
    /// Решение вызываемого пользователя.
    pub response: DirectCallResponse,
}

/// Запрос на отмену исходящего личного звонка до ответа.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelDirectCall {
    /// Идентификатор звонка.
    pub call_id: String,
}

/// Запрос на завершение активного личного звонка.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndDirectCall {
    /// Идентификатор звонка.
    pub call_id: String,
}

/// Запрос актуальных личных звонков пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDirectCalls;

/// Состояние личного звонка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectCallState {
    /// Инициатор ожидает ответа вызываемого пользователя.
    Ringing,
    /// Оба пользователя участвуют в звонке.
    Active,
    /// Звонок завершён.
    Ended,
}

/// Причина завершения личного звонка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectCallEndReason {
    /// Инициатор отменил звонок до ответа.
    Cancelled,
    /// Вызываемый пользователь отклонил звонок.
    Declined,
    /// Вызываемый пользователь не ответил за отведённое время.
    TimedOut,
    /// Один из участников завершил активный звонок.
    Ended,
}

/// Снимок состояния одного личного звонка.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectCallSnapshot {
    /// Стабильный UUID звонка.
    pub call_id: String,
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Идентификатор пользователя, начавшего звонок.
    pub caller_user_id: String,
    /// Снимок имени пользователя, начавшего звонок.
    pub caller_nickname: String,
    /// Публичный URL изображения аватара инициатора, если он настроен.
    pub caller_avatar_url: Option<String>,
    /// Идентификатор вызываемого пользователя.
    pub callee_user_id: String,
    /// Снимок имени вызываемого пользователя.
    pub callee_nickname: String,
    /// Публичный URL изображения аватара вызываемого пользователя, если он настроен.
    pub callee_avatar_url: Option<String>,
    /// Текущее состояние звонка.
    pub state: DirectCallState,
    /// Момент начала звонка в формате RFC3339.
    pub started_at: String,
    /// Момент принятия звонка в формате RFC3339, если звонок был принят.
    pub answered_at: Option<String>,
    /// Момент завершения звонка в формате RFC3339, если звонок завершён.
    pub ended_at: Option<String>,
    /// Причина завершения звонка, если звонок завершён.
    pub end_reason: Option<DirectCallEndReason>,
}

/// Снимок актуальных личных звонков пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectCallsSnapshot {
    /// Незавершённые личные звонки, доступные пользователю.
    pub calls: Vec<DirectCallSnapshot>,
}

/// Адресное событие изменения состояния личного звонка.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectCallLifecycleEvent {
    /// Пользователь, которому предназначено событие.
    pub recipient_user_id: String,
    /// Актуальный снимок изменившегося звонка.
    pub call: DirectCallSnapshot,
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
