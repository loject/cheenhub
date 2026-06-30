//! Вспомогательная сборка REST-ответов для social-сценариев.

use cheenhub_contracts::rest::{
    DmConversationSummary, DmMessageSummary, FriendRequestStatus, FriendRequestSummary,
    FriendSummary, ListFriendRequestsResponse,
};
use uuid::Uuid;

use crate::features::auth::application::auth_user;
use crate::features::auth::domain::UserAccount;
use crate::features::auth::error::AuthError;
use crate::features::social::domain::{DmConversation, DmMessage, Friendship, FriendshipStatus};
use crate::features::social::error::SocialError;
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
    for friendship in friendships {
        let friend_user_id = other_friend_id(&friendship, current_user_id);
        let friend = auth_user(state, &ensure_user_exists(state, &friend_user_id).await?);
        summaries.push(FriendSummary {
            user_id: friend.id,
            nickname: friend.nickname,
            avatar_url: friend.avatar_url,
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
    Ok(DmConversationSummary {
        id: conversation.id.to_string(),
        friend_user_id: friend.id,
        friend_nickname: friend.nickname,
        friend_avatar_url: friend.avatar_url,
        updated_at: conversation.updated_at.to_rfc3339(),
    })
}

pub(super) async fn message_summaries(
    state: &AppState,
    messages: Vec<DmMessage>,
) -> Result<Vec<DmMessageSummary>, SocialError> {
    let mut summaries = Vec::new();
    for message in messages {
        summaries.push(message_summary(state, message).await?);
    }
    Ok(summaries)
}

pub(super) async fn message_summary(
    state: &AppState,
    message: DmMessage,
) -> Result<DmMessageSummary, SocialError> {
    let sender = auth_user(
        state,
        &ensure_user_exists(state, &message.sender_user_id).await?,
    );
    Ok(DmMessageSummary {
        id: message.id.to_string(),
        conversation_id: message.conversation_id.to_string(),
        sender_user_id: sender.id,
        sender_nickname: sender.nickname,
        sender_avatar_url: sender.avatar_url,
        body: message.body,
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
