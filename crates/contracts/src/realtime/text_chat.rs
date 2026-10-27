//! Контракты realtime-модуля текстового чата.

use serde::{Deserialize, Serialize};

use crate::rest::DmMessageDeliveryStatus;

use super::typing::TypingAuthor;

/// Виды сообщений модуля текстового чата.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextChatKind {
    /// Загрузить свежую историю сообщений комнаты.
    LoadRoomHistory,
    /// Ответ с историей сообщений комнаты.
    RoomHistory,
    /// Разрешить роли отдельных авторов в доступной текстовой комнате.
    ResolveAuthorRoles,
    /// Ответ с текущими назначениями запрошенных авторов.
    AuthorRolesResolved,
    /// Сигнал об изменении ролей авторов сервера для обновления кэша.
    AuthorRolesChanged,
    /// Отправить сообщение в комнату.
    SendMessage,
    /// Подтверждает, что сообщение принято для рассылки и сохранения.
    SendMessageAccepted,
    /// Загрузить вложение-изображение чата.
    UploadImage,
    /// Подтверждает, что изображение чата загружено.
    UploadImageAccepted,
    /// Загрузить вложение-изображение чата через realtime.
    LoadImage,
    /// Ответ с одним вложением-изображением чата.
    ImageLoaded,
    /// Событие о новом сообщении.
    MessageCreated,
    /// Изменить текст собственного сообщения.
    EditMessage,
    /// Подтверждает сохранение изменений текста сообщения.
    EditMessageAccepted,
    /// Сообщение отредактировано автором.
    MessageEdited,
    /// Удалить одно из собственных сообщений пользователя.
    DeleteMessage,
    /// Подтверждает, что удаление сообщения принято.
    DeleteMessageAccepted,
    /// Сообщение удалено автором; получатели должны убрать его.
    MessageDeleted,
    /// Запросить закрепления комнаты.
    ListPinnedMessages,
    /// Ответ со списком закреплений комнаты.
    PinnedMessages,
    /// Закрепить сообщение.
    PinMessage,
    /// Подтверждение закрепления.
    PinMessageAccepted,
    /// Снять закрепление.
    UnpinMessage,
    /// Подтверждение снятия закрепления.
    UnpinMessageAccepted,
    /// Событие закрепления сообщения.
    MessagePinned,
    /// Событие снятия закрепления.
    MessageUnpinned,
    /// Сообщить, что пользователь печатает сообщение в комнате.
    StartTyping,
    /// Сообщить, что пользователь перестал печатать сообщение в комнате.
    StopTyping,
    /// Запросить снимок участников комнаты, которые сейчас печатают.
    TypingSnapshot,
    /// Событие о начале или завершении набора сообщения в комнате.
    TypingChanged,
}

/// Полезная нагрузка запроса для загрузки истории комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRoomHistory {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор сообщения, перед которым нужно загрузить сообщения.
    pub before_message_id: Option<String>,
}

/// Полезная нагрузка ответа с последними сообщениями комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomHistory {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Последние сохраненные сообщения комнаты.
    pub messages: Vec<TextChatMessage>,
    /// Доступны ли более старые сообщения до этой страницы.
    pub has_more: bool,
    /// Текущие роли уникальных авторов сообщений этой страницы.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub author_role_ids: std::collections::HashMap<String, Vec<String>>,
}

/// Запрос ролей только выбранных авторов доступной текстовой комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveAuthorRoles {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор текстовой комнаты для проверки права чтения.
    pub room_id: String,
    /// Не более 100 идентификаторов авторов на один запрос.
    pub author_user_ids: Vec<String>,
}

/// Текущие роли активных авторов, которые присутствуют в запрошенной выборке.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorRolesResolved {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Назначения только активных авторов; отсутствующие считаются без ролей.
    pub author_role_ids: std::collections::HashMap<String, Vec<String>>,
}

/// Полезная нагрузка запроса для отправки сообщения в комнату.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Тело сообщения.
    pub body: String,
    /// Идентификаторы загруженных вложений-изображений, которые нужно включить в сообщение.
    #[serde(default)]
    pub attachment_ids: Vec<String>,
}

/// Полезная нагрузка ответа после принятия отправки сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessageAccepted {
    /// Принятое сообщение.
    pub message: TextChatMessage,
}

/// Запрос редактирования текста существующего сообщения комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditMessage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор редактируемого сообщения.
    pub message_id: String,
    /// Новый текст; пустой допускается только при наличии вложения.
    pub body: String,
}

/// Подтверждение сохранения отредактированного сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditMessageAccepted {
    /// Актуальный снимок сообщения со временем редактирования.
    pub message: TextChatMessage,
}

/// Полезная нагрузка запроса для загрузки одного вложения-изображения чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadChatImage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Необязательное исходное имя файла.
    pub original_filename: Option<String>,
    /// Байты изображения в Base64.
    pub data_base64: String,
}

/// Ответ после загрузки изображения чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatImageUploadResponse {
    /// Стабильный идентификатор вложения.
    pub id: String,
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Проверенный MIME-тип изображения.
    pub content_type: String,
    /// Загруженный размер в байтах.
    pub byte_size: i64,
    /// Ширина изображения в пикселях.
    pub width: i32,
    /// Высота изображения в пикселях.
    pub height: i32,
}

/// Метаданные вложения-изображения, включаемые в сообщение текстового чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextChatImageAttachment {
    /// Стабильный идентификатор вложения.
    pub id: String,
    /// Проверенный MIME-тип изображения.
    pub content_type: String,
    /// Загруженный размер в байтах.
    pub byte_size: i64,
    /// Ширина изображения в пикселях.
    pub width: i32,
    /// Высота изображения в пикселях.
    pub height: i32,
}

/// Полезная нагрузка запроса для загрузки одного вложения-изображения чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadChatImage {
    /// Стабильный идентификатор вложения.
    pub attachment_id: String,
}

/// Полезная нагрузка ответа с одним вложением-изображением чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatImageLoadedResponse {
    /// Стабильный идентификатор вложения.
    pub id: String,
    /// Проверенный MIME-тип изображения.
    pub content_type: String,
    /// Байты изображения в Base64.
    pub data_base64: String,
}

/// Полезная нагрузка запроса для мягкого удаления одного из собственных сообщений пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteMessage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор сообщения для удаления.
    pub message_id: String,
}

/// Полезная нагрузка ответа после принятия удаления сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteMessageAccepted {
    /// Идентификатор удаленного сообщения.
    pub message_id: String,
}

/// Полезная нагрузка широковещания, уведомляющая участников комнаты об удалении сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeletedPayload {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор удаленного сообщения.
    pub message_id: String,
}

/// Запрос списка закреплений комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPinnedMessages {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
}

/// Запрос закрепления сообщения в комнате.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinMessage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор сообщения.
    pub message_id: String,
}

/// Запрос снятия закрепления сообщения в комнате.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnpinMessage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор сообщения.
    pub message_id: String,
}

/// Сообщение с метаданными закрепления.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedTextMessage {
    /// Полный снимок исходного сообщения.
    pub message: TextChatMessage,
    /// Пользователь, закрепивший сообщение.
    pub pinned_by_user_id: String,
    /// Время закрепления в формате RFC3339.
    pub pinned_at: String,
}

/// Ответ со всеми закреплениями комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedMessages {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Закрепления от новых к старым.
    pub messages: Vec<PinnedTextMessage>,
    /// Максимальное количество закреплений в комнате.
    pub limit: u32,
    /// Текущие роли уникальных авторов закреплённых сообщений.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub author_role_ids: std::collections::HashMap<String, Vec<String>>,
}

/// Подтверждение добавления закрепления.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinMessageAccepted {
    /// Сохранённое закрепление.
    pub pinned: PinnedTextMessage,
}

/// Подтверждение снятия закрепления, также используется в realtime-событии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnpinMessageAccepted {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор сообщения.
    pub message_id: String,
}

/// Полезная нагрузка сообщения текстового чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextChatMessage {
    /// Стабильный идентификатор сообщения.
    pub id: String,
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор пользователя-автора.
    pub author_user_id: String,
    /// Снимок ника автора.
    pub author_nickname: String,
    /// Публичный URL аватара автора, если он настроен.
    pub author_avatar_url: Option<String>,
    /// Тело сообщения.
    pub body: String,
    /// Вложения-изображения, включенные в сообщение.
    #[serde(default)]
    pub attachments: Vec<TextChatImageAttachment>,
    /// Статус доставки для личных сообщений.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_status: Option<DmMessageDeliveryStatus>,
    /// Временная метка создания сообщения в формате RFC3339.
    pub created_at: String,
    /// Время последнего изменения текста в формате RFC3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
}

/// Полезная нагрузка запроса о начале набора сообщения в комнате.
///
/// Отправляется один раз при переходе из состояния «не печатает» в состояние
/// «печатает» и повторяется как продление, пока пользователь печатает.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartTyping {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
}

/// Полезная нагрузка запроса о завершении набора сообщения в комнате.
///
/// Отправляется после отправки сообщения, очистки черновика, потери фокуса
/// формы или закрытия вкладки, чтобы другие участники быстро увидели, что
/// пользователь перестал печатать.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopTyping {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
}

/// Полезная нагрузка запроса снимка участников комнаты, которые печатают.
///
/// Нужна при открытии комнаты и после переподключения realtime, чтобы показать
/// текущее состояние набора, а не ждать следующего события от других участников.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypingSnapshotRequest {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
}

/// Полезная нагрузка ответа со снимком участников комнаты, которые печатают.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypingSnapshot {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Участники, которые печатают сообщение прямо сейчас.
    pub typers: Vec<TypingAuthor>,
}

/// Полезная нагрузка события о начале или завершении набора в комнате.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypingChanged {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Участник, который начал или закончил печатать.
    pub author: TypingAuthor,
    /// Признак того, что участник печатает сообщение.
    pub is_typing: bool,
}
