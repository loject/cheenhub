//! Вспомогательная сборка REST-ответов для social-сценариев.

use std::collections::HashMap;

use cheenhub_contracts::rest::{
    AuthUser, DmConversationSummary, DmImageAttachmentSummary, DmLastMessageSummary,
    DmMessageDeliveryStatus, DmMessageSummary, FriendRequestStatus, FriendRequestSummary,
    FriendSummary, ListFriendRequestsResponse,
};
use chrono::Utc;
use uuid::Uuid;

use crate::features::auth::application::auth_user;
use crate::features::auth::domain::UserAccount;
use crate::features::auth::error::AuthError;
use crate::features::social::domain::{
    ConversationMemberState, DmConversation, DmMessage, FriendListEntry, Friendship,
    FriendshipStatus,
};
use crate::features::social::error::SocialError;
use crate::features::social::infrastructure::normalize_unread_count;
use crate::state::AppState;

pub(super) async fn request_response(
    state: &AppState,
    requests: Vec<Friendship>,
) -> Result<ListFriendRequestsResponse, SocialError> {
    let mut summaries = Vec::new();
    for request in requests {
        summaries.push(request_summary(state, request).await?);
    }
    Ok(ListFriendRequestsResponse {
        requests: summaries,
    })
}

pub(super) async fn friend_summaries(
    state: &AppState,
    entries: Vec<FriendListEntry>,
) -> Result<Vec<FriendSummary>, SocialError> {
    let user_ids = entries
        .iter()
        .map(|entry| entry.friend_user_id)
        .collect::<Vec<_>>();
    let users = state
        .auth_store
        .find_users_by_ids(&user_ids)
        .await
        .map_err(SocialError::Internal)?;
    let users = users
        .into_iter()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();
    let mut summaries = Vec::with_capacity(entries.len());
    for entry in entries {
        let friend = users.get(&entry.friend_user_id).ok_or_else(|| {
            SocialError::NotFound("Пользователь больше не существует.".to_owned())
        })?;
        let friend = auth_user(state, friend);
        summaries.push(FriendSummary {
            user_id: friend.id,
            nickname: friend.nickname,
            avatar_url: friend.avatar_url,
            unread_count: normalize_unread_count(entry.unread_count),
            last_message: entry.last_message.map(|message| DmLastMessageSummary {
                id: message.id.to_string(),
                sender_user_id: message.sender_user_id.to_string(),
                body: message.body,
                has_image: message.image_id.is_some(),
                created_at: message.created_at.to_rfc3339(),
            }),
            friends_since: entry.friendship.updated_at.to_rfc3339(),
        });
    }
    Ok(summaries)
}

pub(super) async fn request_summary(
    state: &AppState,
    friendship: Friendship,
) -> Result<FriendRequestSummary, SocialError> {
    let sender = auth_user(
        state,
        &ensure_user_exists(state, &friendship.requester_user_id).await?,
    );
    let recipient = auth_user(
        state,
        &ensure_user_exists(state, &friendship.recipient_user_id).await?,
    );
    Ok(FriendRequestSummary {
        id: friendship.id.to_string(),
        sender_user_id: sender.id,
        sender_nickname: sender.nickname,
        sender_avatar_url: sender.avatar_url,
        recipient_user_id: recipient.id,
        recipient_nickname: recipient.nickname,
        recipient_avatar_url: recipient.avatar_url,
        status: request_status(friendship.status),
        created_at: friendship.created_at.to_rfc3339(),
        updated_at: friendship.updated_at.to_rfc3339(),
    })
}

pub(super) async fn conversation_summaries(
    state: &AppState,
    current_user_id: &Uuid,
    conversations: Vec<DmConversation>,
) -> Result<Vec<DmConversationSummary>, SocialError> {
    let friends = auth_users_by_id(
        state,
        conversations
            .iter()
            .map(|conversation| other_user_id(conversation, current_user_id)),
    )
    .await?;
    let member_states = state
        .social_store
        .conversation_member_states_for_user(current_user_id)
        .await
        .map_err(SocialError::Internal)?
        .into_iter()
        .map(|state| (state.conversation_id, state))
        .collect::<HashMap<_, _>>();

    let mut summaries = Vec::with_capacity(conversations.len());
    for conversation in conversations {
        let friend_user_id = other_user_id(&conversation, current_user_id);
        let friend = friends
            .get(&friend_user_id)
            .cloned()
            .ok_or_else(|| SocialError::NotFound("Пользователь не найден.".to_owned()))?;
        let member_state = member_states
            .get(&conversation.id)
            .cloned()
            .unwrap_or_else(|| default_member_state(&conversation, current_user_id));
        summaries.push(conversation_summary_from_parts(
            friend,
            conversation,
            member_state,
        ));
    }
    Ok(summaries)
}

/// Собирает сводку диалога из уже загруженных собеседника и read-state.
///
/// Вынесено отдельно от `conversation_summary`, чтобы пакетная и одиночная сборки
/// гарантированно строили одинаковую структуру ответа.
fn conversation_summary_from_parts(
    friend: AuthUser,
    conversation: DmConversation,
    member_state: ConversationMemberState,
) -> DmConversationSummary {
    DmConversationSummary {
        id: conversation.id.to_string(),
        friend_user_id: friend.id,
        friend_nickname: friend.nickname,
        friend_avatar_url: friend.avatar_url,
        unread_count: normalize_unread_count(member_state.unread_count),
        last_read_message_id: member_state
            .last_read_message_id
            .map(|message_id| message_id.to_string()),
        last_read_seq: member_state.last_read_seq,
        last_read_at: member_state
            .last_read_at
            .map(|read_at| read_at.to_rfc3339()),
        updated_at: conversation.updated_at.to_rfc3339(),
    }
}

pub(super) async fn conversation_summary(
    state: &AppState,
    current_user_id: &Uuid,
    conversation: DmConversation,
) -> Result<DmConversationSummary, SocialError> {
    let friend_user_id = other_user_id(&conversation, current_user_id);
    let friend = auth_user(state, &ensure_user_exists(state, &friend_user_id).await?);
    let member_state = state
        .social_store
        .conversation_member_state(&conversation.id, current_user_id)
        .await
        .map_err(SocialError::Internal)?
        .unwrap_or_else(|| default_member_state(&conversation, current_user_id));
    Ok(conversation_summary_from_parts(
        friend,
        conversation,
        member_state,
    ))
}

/// Загружает пользователей страницы одним обращением к хранилищу, индексируя по идентификатору.
///
/// Пагинация собирается по одному пользователю на элемент, поэтому построчный
/// `ensure_user_exists` давал бы N+1 запросов на страницу.
async fn auth_users_by_id(
    state: &AppState,
    user_ids: impl IntoIterator<Item = Uuid>,
) -> Result<HashMap<Uuid, AuthUser>, SocialError> {
    let user_ids = user_ids.into_iter().collect::<Vec<_>>();
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let users = state
        .auth_store
        .find_users_by_ids(&user_ids)
        .await
        .map_err(SocialError::Internal)?;
    Ok(users
        .into_iter()
        .map(|user| (user.id, auth_user(state, &user)))
        .collect())
}

/// Собирает сводки страницы сообщений, загружая отправителей и вложения пачками.
///
/// Отправитель и картинка нужны для каждого сообщения, но почти всегда повторяются,
/// поэтому один вызов на страницу заменяет два запроса на каждое сообщение.
pub(super) async fn message_summaries(
    state: &AppState,
    current_user_id: &Uuid,
    recipient_last_read_seq: i64,
    messages: Vec<DmMessage>,
) -> Result<Vec<DmMessageSummary>, SocialError> {
    let senders =
        auth_users_by_id(state, messages.iter().map(|message| message.sender_user_id)).await?;
    let images = super::application::attachment_summaries_by_conversation(
        state,
        &messages
            .iter()
            .map(|message| (message.conversation_id, message.image_id))
            .collect::<Vec<_>>(),
    )
    .await?;

    let mut summaries = Vec::with_capacity(messages.len());
    for message in messages {
        let sender = senders
            .get(&message.sender_user_id)
            .cloned()
            .ok_or_else(|| SocialError::NotFound("Пользователь не найден.".to_owned()))?;
        let image = message
            .image_id
            .and_then(|image_id| images.get(&(message.conversation_id, image_id)).cloned());
        summaries.push(message_summary_from_parts(
            sender,
            image,
            current_user_id,
            recipient_last_read_seq,
            message,
        ));
    }
    Ok(summaries)
}

pub(super) async fn message_summary(
    state: &AppState,
    current_user_id: &Uuid,
    recipient_last_read_seq: i64,
    message: DmMessage,
) -> Result<DmMessageSummary, SocialError> {
    let sender = auth_user(
        state,
        &ensure_user_exists(state, &message.sender_user_id).await?,
    );
    let image =
        super::application::attachment_summary(state, message.conversation_id, message.image_id)
            .await?;
    Ok(message_summary_from_parts(
        sender,
        image,
        current_user_id,
        recipient_last_read_seq,
        message,
    ))
}

/// Собирает сводку сообщения из уже загруженных отправителя и вложения.
///
/// Вынесено отдельно от `message_summary`, чтобы пакетная и одиночная сборки
/// гарантированно строили одинаковую структуру ответа.
fn message_summary_from_parts(
    sender: AuthUser,
    image: Option<DmImageAttachmentSummary>,
    current_user_id: &Uuid,
    recipient_last_read_seq: i64,
    message: DmMessage,
) -> DmMessageSummary {
    DmMessageSummary {
        id: message.id.to_string(),
        conversation_id: message.conversation_id.to_string(),
        seq: message.seq,
        sender_user_id: sender.id,
        sender_nickname: sender.nickname,
        sender_avatar_url: sender.avatar_url,
        delivery_status: delivery_status(&message, current_user_id, recipient_last_read_seq),
        body: message.body,
        image,
        created_at: message.created_at.to_rfc3339(),
    }
}

pub(super) async fn load_user_conversation(
    state: &AppState,
    conversation_id: &Uuid,
    user_id: &Uuid,
) -> Result<DmConversation, SocialError> {
    let conversation = state
        .social_store
        .conversation_by_id(conversation_id)
        .await
        .map_err(SocialError::Internal)?
        .ok_or_else(|| SocialError::NotFound("Диалог не найден.".to_owned()))?;
    if conversation.user_low_id == *user_id || conversation.user_high_id == *user_id {
        Ok(conversation)
    } else {
        Err(SocialError::NotFound("Диалог не найден.".to_owned()))
    }
}

/// Запрещает новые взаимодействия с аккаунтом, помеченным для удаления.
pub(super) async fn ensure_user_active(
    state: &AppState,
    user_id: &Uuid,
) -> Result<(), SocialError> {
    if state
        .auth_store
        .account_deletion(user_id)
        .await
        .map_err(SocialError::Internal)?
        .is_some()
    {
        tracing::warn!(%user_id, "rejected social action targeting deleted account");
        return Err(SocialError::BadRequest("Этот аккаунт удалён.".to_owned()));
    }
    Ok(())
}

pub(super) async fn ensure_user_exists(
    state: &AppState,
    user_id: &Uuid,
) -> Result<UserAccount, SocialError> {
    state
        .auth_store
        .find_user_by_id(user_id)
        .await
        .map_err(SocialError::Internal)?
        .ok_or_else(|| SocialError::NotFound("Пользователь не найден.".to_owned()))
}

pub(super) fn other_user_id(conversation: &DmConversation, current_user_id: &Uuid) -> Uuid {
    if conversation.user_low_id == *current_user_id {
        conversation.user_high_id
    } else {
        conversation.user_low_id
    }
}

pub(super) fn parse_id(value: &str, message: &str) -> Result<Uuid, SocialError> {
    Uuid::parse_str(value).map_err(|_| SocialError::BadRequest(message.to_owned()))
}

pub(super) fn message_body(body: String) -> Result<String, SocialError> {
    let body = body.trim().to_owned();
    if body.is_empty() {
        return Err(SocialError::BadRequest(
            "Сообщение не может быть пустым.".to_owned(),
        ));
    }
    if body.chars().count() > 4000 {
        return Err(SocialError::BadRequest(
            "Сообщение слишком длинное.".to_owned(),
        ));
    }
    Ok(body)
}

pub(super) fn map_auth_error(error: AuthError) -> SocialError {
    match error {
        AuthError::BadRequest(message) | AuthError::Unauthorized(message) => {
            SocialError::Unauthorized(message)
        }
        AuthError::RefreshRejected { message, .. }
        | AuthError::RefreshRotationInProgress(message) => SocialError::Unauthorized(message),
        AuthError::Conflict(message) | AuthError::RateLimited(message) => {
            SocialError::BadRequest(message)
        }
        AuthError::Misconfigured { message, .. } => SocialError::Internal(anyhow::anyhow!(message)),
        AuthError::Internal(error) => SocialError::Internal(error),
    }
}

fn request_status(status: FriendshipStatus) -> FriendRequestStatus {
    match status {
        FriendshipStatus::Pending => FriendRequestStatus::Pending,
        FriendshipStatus::Accepted => FriendRequestStatus::Accepted,
        FriendshipStatus::Declined => FriendRequestStatus::Declined,
        FriendshipStatus::Cancelled => FriendRequestStatus::Cancelled,
    }
}

fn default_member_state(
    conversation: &DmConversation,
    current_user_id: &Uuid,
) -> ConversationMemberState {
    ConversationMemberState {
        conversation_id: conversation.id,
        user_id: *current_user_id,
        last_read_message_id: None,
        last_read_seq: 0,
        last_read_at: None,
        unread_count: 0,
        updated_at: Utc::now(),
    }
}

fn delivery_status(
    message: &DmMessage,
    current_user_id: &Uuid,
    recipient_last_read_seq: i64,
) -> Option<DmMessageDeliveryStatus> {
    if message.sender_user_id != *current_user_id {
        return None;
    }
    if message.seq <= recipient_last_read_seq {
        Some(DmMessageDeliveryStatus::Read)
    } else {
        Some(DmMessageDeliveryStatus::Accepted)
    }
}
