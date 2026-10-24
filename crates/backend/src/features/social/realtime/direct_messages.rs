//! Команды личных сообщений: декодирование, вызов application и ответы Social.
//!
//! Аутентификация принадлежит realtime-сессии, проверки доступа — application.
//! Ошибки команды возвращаются как отказ и не закрывают исправный поток.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cheenhub_contracts::realtime::{
    DirectMessageImageLoaded, EditDirectMessage, ListDirectMessageConversations,
    LoadDirectMessageHistory, LoadDirectMessageImage, MarkDirectMessageRead,
    OpenDirectMessageConversation, RealtimeEnvelope, RealtimeKind, RealtimeModule, RejectionCode,
    SendDirectMessage, SocialKind, UploadDirectMessageImage,
};
use cheenhub_contracts::rest::{
    EditDmMessageRequest, MarkDmConversationReadRequest, OpenDmConversationRequest,
    SendDmMessageRequest,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;

use crate::features::social::{application, error::SocialError};
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{send_rejection, write_envelope};
use crate::state::AppState;

/// Выполняет команду ЛС от имени пользователя, проверенного realtime-сессией.
///
/// Требует `request_id` до выполнения операции; отказ сохраняет корреляцию.
pub(super) async fn handle(
    state: &AppState,
    user_id: &Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    let Some(request_id) = envelope.request_id else {
        return reject(
            send,
            user_id,
            &envelope,
            SocialError::BadRequest("Для команды личных сообщений нужен request_id.".to_owned()),
        )
        .await;
    };
    match execute(state, user_id, &envelope).await {
        Ok((kind, payload)) => {
            tracing::debug!(%user_id, %request_id, kind = ?envelope.kind, response_kind = ?kind, "completed direct message realtime command");
            write_envelope(
                send,
                RealtimeModule::Social,
                RealtimeKind::Social(kind),
                Some(request_id),
                payload,
            )
            .await
        }
        Err(error) => reject(send, user_id, &envelope, error).await,
    }
}

async fn execute(
    state: &AppState,
    user_id: &Uuid,
    envelope: &RealtimeEnvelope,
) -> Result<(SocialKind, Value), SocialError> {
    match envelope.kind {
        RealtimeKind::Social(SocialKind::ListDirectMessageConversations) => {
            let _: ListDirectMessageConversations = decode(envelope)?;
            let result = application::list_dm_conversations(state, user_id).await?;
            response(SocialKind::DirectMessageConversations, result)
        }
        RealtimeKind::Social(SocialKind::OpenDirectMessageConversation) => {
            let request: OpenDirectMessageConversation = decode(envelope)?;
            let result = application::open_dm_conversation(
                state,
                user_id,
                OpenDmConversationRequest {
                    friend_user_id: request.friend_user_id,
                },
            )
            .await?;
            response(SocialKind::DirectMessageConversationOpened, result)
        }
        RealtimeKind::Social(SocialKind::LoadDirectMessageHistory) => {
            let request: LoadDirectMessageHistory = decode(envelope)?;
            let result = application::list_dm_messages(
                state,
                user_id,
                request.conversation_id,
                request.before_message_id,
            )
            .await?;
            response(SocialKind::DirectMessageHistory, result)
        }
        RealtimeKind::Social(SocialKind::SendDirectMessage) => {
            let request: SendDirectMessage = decode(envelope)?;
            let result = application::send_dm_message(
                state,
                user_id,
                request.conversation_id,
                SendDmMessageRequest {
                    body: request.body,
                    image_id: request.image_id,
                },
            )
            .await?;
            response(SocialKind::SendDirectMessageAccepted, result)
        }
        RealtimeKind::Social(SocialKind::EditDirectMessage) => {
            let request: EditDirectMessage = decode(envelope)?;
            let result = application::edit_dm_message(
                state,
                user_id,
                request.conversation_id,
                request.message_id,
                EditDmMessageRequest { body: request.body },
            )
            .await?;
            response(SocialKind::EditDirectMessageAccepted, result)
        }
        RealtimeKind::Social(SocialKind::MarkDirectMessageRead) => {
            let request: MarkDirectMessageRead = decode(envelope)?;
            let result = application::mark_dm_conversation_read(
                state,
                user_id,
                request.conversation_id,
                MarkDmConversationReadRequest {
                    last_read_message_id: request.last_read_message_id,
                },
            )
            .await?;
            response(SocialKind::MarkDirectMessageReadAccepted, result)
        }
        RealtimeKind::Social(SocialKind::UploadDirectMessageImage) => {
            let request: UploadDirectMessageImage = decode(envelope)?;
            let bytes = BASE64.decode(request.data_base64).map_err(|_| {
                SocialError::BadRequest("Не удалось прочитать изображение.".to_owned())
            })?;
            let result =
                application::upload_dm_image(state, user_id, request.conversation_id, bytes.into())
                    .await?;
            response(SocialKind::UploadDirectMessageImageAccepted, result)
        }
        RealtimeKind::Social(SocialKind::LoadDirectMessageImage) => {
            let request: LoadDirectMessageImage = decode(envelope)?;
            let image =
                application::dm_image(state, user_id, request.conversation_id, request.image_id)
                    .await?;
            let data = image
                .data
                .ok_or_else(|| SocialError::NotFound("Изображение не найдено.".to_owned()))?;
            response(
                SocialKind::DirectMessageImageLoaded,
                DirectMessageImageLoaded {
                    id: image.id.to_string(),
                    content_type: image.content_type,
                    data_base64: BASE64.encode(data),
                },
            )
        }
        _ => Err(SocialError::BadRequest(
            "Неизвестная команда личных сообщений.".to_owned(),
        )),
    }
}

fn decode<T: DeserializeOwned>(envelope: &RealtimeEnvelope) -> Result<T, SocialError> {
    serde_json::from_value(envelope.payload.clone()).map_err(|_| {
        SocialError::BadRequest("Не удалось прочитать запрос личных сообщений.".to_owned())
    })
}

fn response<T: Serialize>(
    kind: SocialKind,
    payload: T,
) -> Result<(SocialKind, Value), SocialError> {
    serde_json::to_value(payload)
        .map(|value| (kind, value))
        .map_err(|error| SocialError::Internal(error.into()))
}

async fn reject(
    send: &EnvelopeSink,
    user_id: &Uuid,
    envelope: &RealtimeEnvelope,
    error: SocialError,
) -> anyhow::Result<()> {
    let (code, message) = match error {
        SocialError::BadRequest(message)
        | SocialError::NotFound(message)
        | SocialError::Conflict(message) => (RejectionCode::BadRequest, message),
        SocialError::Unauthorized(message) => (RejectionCode::Unauthorized, message),
        SocialError::Internal(error) => {
            tracing::error!(%user_id, request_id = ?envelope.request_id, kind = ?envelope.kind, %error, "direct message realtime command failed");
            return send_rejection(
                send,
                envelope.request_id,
                RejectionCode::InternalError,
                "Не удалось выполнить действие личных сообщений.",
            )
            .await;
        }
    };
    tracing::warn!(%user_id, request_id = ?envelope.request_id, kind = ?envelope.kind, ?code, "rejected direct message realtime command");
    send_rejection(send, envelope.request_id, code, &message).await
}
