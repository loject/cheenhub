//! Команды ЛС через общий realtime с резервным WebSocket-транспортом.
//!
//! Постоянный Social-поток разделяет ответы по `request_id` и события подписки.
//! Отказы сервера остаются пользовательскими сообщениями, технические ошибки
//! соединения попадают в логи и заменяются понятным предложением повторить действие.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cheenhub_contracts::realtime::{
    DirectMessageConversationOpened, DirectMessageConversations, DirectMessageHistory,
    DirectMessageImageLoaded, EditDirectMessage, EditDirectMessageAccepted,
    ListDirectMessageConversations, LoadDirectMessageHistory, LoadDirectMessageImage,
    MarkDirectMessageRead, MarkDirectMessageReadAccepted, OpenDirectMessageConversation,
    RealtimeKind, RealtimeModule, SendDirectMessage, SendDirectMessageAccepted, SocialKind,
    UploadDirectMessageImage, UploadDirectMessageImageAccepted,
};
use cheenhub_contracts::rest::{DmConversationSummary, DmImageAttachmentSummary, DmMessageSummary};
use dioxus::prelude::warn;
use serde::{Serialize, de::DeserializeOwned};

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Загружает список личных диалогов текущего realtime-пользователя.
pub(crate) async fn list_dm_conversations(
    realtime: &RealtimeHandle,
) -> Result<Vec<DmConversationSummary>, String> {
    request::<_, DirectMessageConversations>(
        realtime,
        SocialKind::ListDirectMessageConversations,
        ListDirectMessageConversations,
        "Не удалось загрузить диалоги. Проверь соединение и повтори попытку.",
    )
    .await
    .map(|response| response.conversations)
}

/// Открывает личный диалог с другим пользователем; дружба не обязательна.
pub(crate) async fn open_dm_conversation(
    realtime: &RealtimeHandle,
    friend_user_id: String,
) -> Result<DmConversationSummary, String> {
    request::<_, DirectMessageConversationOpened>(
        realtime,
        SocialKind::OpenDirectMessageConversation,
        OpenDirectMessageConversation { friend_user_id },
        "Не удалось открыть диалог. Проверь соединение и повтори попытку.",
    )
    .await
    .map(|response| response.conversation)
}

/// Загружает последнюю или более старую страницу сообщений.
///
/// `before_message_id` задаёт курсор перед первым уже загруженным сообщением.
pub(crate) async fn list_dm_messages(
    realtime: &RealtimeHandle,
    conversation_id: &str,
    before_message_id: Option<&str>,
) -> Result<DirectMessageHistory, String> {
    request(
        realtime,
        SocialKind::LoadDirectMessageHistory,
        LoadDirectMessageHistory {
            conversation_id: conversation_id.to_owned(),
            before_message_id: before_message_id.map(ToOwned::to_owned),
        },
        "Не удалось загрузить сообщения. Проверь соединение и повтори попытку.",
    )
    .await
}

/// Подтверждает прочтение диалога до указанного сообщения.
pub(crate) async fn mark_dm_conversation_read(
    realtime: &RealtimeHandle,
    conversation_id: &str,
    last_read_message_id: String,
) -> Result<MarkDirectMessageReadAccepted, String> {
    request(
        realtime,
        SocialKind::MarkDirectMessageRead,
        MarkDirectMessageRead {
            conversation_id: conversation_id.to_owned(),
            last_read_message_id,
        },
        "Не удалось отметить сообщения прочитанными. Проверь соединение и повтори попытку.",
    )
    .await
}

/// Отправляет сообщение и возвращает сохранённые сервером данные.
pub(crate) async fn send_dm_message(
    realtime: &RealtimeHandle,
    conversation_id: &str,
    body: String,
    image_id: Option<String>,
) -> Result<DmMessageSummary, String> {
    request::<_, SendDirectMessageAccepted>(
        realtime,
        SocialKind::SendDirectMessage,
        SendDirectMessage {
            conversation_id: conversation_id.to_owned(),
            body,
            image_id,
        },
        "Не удалось отправить сообщение. Проверь соединение и повтори попытку.",
    )
    .await
    .map(|response| response.message)
}

/// Изменяет текст собственного сообщения, сохраняя серверную проверку авторства.
pub(crate) async fn edit_dm_message(
    realtime: &RealtimeHandle,
    conversation_id: &str,
    message_id: &str,
    body: String,
) -> Result<DmMessageSummary, String> {
    request::<_, EditDirectMessageAccepted>(
        realtime,
        SocialKind::EditDirectMessage,
        EditDirectMessage {
            conversation_id: conversation_id.to_owned(),
            message_id: message_id.to_owned(),
            body,
        },
        "Не удалось сохранить изменения. Проверь соединение и повтори попытку.",
    )
    .await
    .map(|response| response.message)
}

/// Загружает изображение в текущем base64-контракте ЛС.
///
/// Лимит 8 МиБ проверяется до кодирования и отправки запроса.
pub(crate) async fn upload_dm_image(
    realtime: &RealtimeHandle,
    conversation_id: &str,
    bytes: Vec<u8>,
) -> Result<DmImageAttachmentSummary, String> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Изображение слишком большое. Максимум — 8 МБ.".to_owned());
    }
    request::<_, UploadDirectMessageImageAccepted>(
        realtime,
        SocialKind::UploadDirectMessageImage,
        UploadDirectMessageImage {
            conversation_id: conversation_id.to_owned(),
            data_base64: BASE64.encode(bytes),
        },
        "Не удалось загрузить изображение. Проверь соединение и повтори попытку.",
    )
    .await
    .map(|response| response.image)
}

/// Получает base64 изображения для preview без повторного кодирования байтов.
pub(crate) async fn load_dm_image(
    realtime: &RealtimeHandle,
    conversation_id: &str,
    image_id: &str,
) -> Result<String, String> {
    request::<_, DirectMessageImageLoaded>(
        realtime,
        SocialKind::LoadDirectMessageImage,
        LoadDirectMessageImage {
            conversation_id: conversation_id.to_owned(),
            image_id: image_id.to_owned(),
        },
        "Не удалось загрузить изображение. Проверь соединение и повтори попытку.",
    )
    .await
    .map(|response| response.data_base64)
}

async fn request<P: Serialize, R: DeserializeOwned>(
    realtime: &RealtimeHandle,
    kind: SocialKind,
    payload: P,
    failure: &'static str,
) -> Result<R, String> {
    realtime
        .request(RealtimeModule::Social, RealtimeKind::Social(kind), payload)
        .await
        .map_err(|error| {
            warn!(?kind, %error, "direct message realtime request failed");
            user_error(&error, failure)
        })
}

fn user_error(error: &RealtimeError, fallback: &str) -> String {
    error.user_message().unwrap_or(fallback).to_owned()
}

#[cfg(test)]
mod tests;
