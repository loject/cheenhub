//! Вспомогательная сборка REST-ответов для social-сценариев.

use cheenhub_contracts::rest::{
    DmConversationSummary, DmMessageDeliveryStatus, DmMessageSummary, FriendRequestStatus,
    FriendRequestSummary, FriendSummary, ListFriendRequestsResponse,
};
use chrono::Utc;
use uuid::Uuid;

use crate::features::auth::application::auth_user;
use crate::features::auth::domain::UserAccount;
use crate::features::auth::error::AuthError;
use crate::features::social::domain::{
    ConversationMemberState, DmConversation, DmMessage, Friendship, FriendshipStatus,
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
    current_user_id: &Uuid,
    friendships: Vec<Friendship>,
) -> Result<Vec<FriendSummary>, SocialError> {
    let mut summaries = Vec::new();
    let conversations = state
        .social_store
        .conversations_for_user(current_user_id)
        .await
        .map_err(SocialError::Internal)?;
    for friendship in friendships {
        let friend_user_id = other_friend_id(&friendship, current_user_id);
        let friend = auth_user(state, &ensure_user_exists(state, &friend_user_id).await?);
        let unread_count =
            friend_unread_count(state, current_user_id, &friend_user_id, &conversations).await?;
        summaries.push(FriendSummary {
            user_id: friend.id,
            nickname: friend.nickname,
            avatar_url: friend.avatar_url,
            unread_count: normalize_unread_count(unread_count),
            friends_since: friendship.updated_at.to_rfc3339(),
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
    let mut summaries = Vec::new();
    for conversation in conversations {
        summaries.push(conversation_summary(state, current_user_id, conversation).await?);
    }
    Ok(summaries)
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
    Ok(DmConversationSummary {
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
    })
}

pub(super) async fn message_summaries(
    state: &AppState,
    current_user_id: &Uuid,
    recipient_last_read_seq: i64,
    messages: Vec<DmMessage>,
) -> Result<Vec<DmMessageSummary>, SocialError> {
    let mut summaries = Vec::new();
    for message in messages {
        summaries
            .push(message_summary(state, current_user_id, recipient_last_read_seq, message).await?);
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
    Ok(DmMessageSummary {
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
    })
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

fn other_friend_id(friendship: &Friendship, current_user_id: &Uuid) -> Uuid {
    if friendship.requester_user_id == *current_user_id {
        friendship.recipient_user_id
    } else {
        friendship.requester_user_id
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

async fn friend_unread_count(
    state: &AppState,
    current_user_id: &Uuid,
    friend_user_id: &Uuid,
    conversations: &[DmConversation],
) -> Result<i64, SocialError> {
    let Some(conversation) = conversations
        .iter()
        .find(|conversation| other_user_id(conversation, current_user_id) == *friend_user_id)
    else {
        return Ok(0);
    };
    Ok(state
        .social_store
        .conversation_member_state(&conversation.id, current_user_id)
        .await
        .map_err(SocialError::Internal)?
        .map(|state| normalize_unread_count(state.unread_count))
        .unwrap_or(0))
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
