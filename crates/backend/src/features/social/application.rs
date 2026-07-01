//! Сценарии приложения для друзей и личных сообщений.

mod direct_messages;

use cheenhub_contracts::realtime::SocialChangeReason;
use cheenhub_contracts::rest::{
    ListFriendRequestsResponse, ListFriendsResponse, SearchUsersResponse, SendFriendRequestRequest,
    SendFriendRequestResponse, UserRelationStatus, UserSearchResult,
};
use uuid::Uuid;

use crate::features::auth::application::{auth_user, require_current_user};
use crate::features::social::domain::FriendshipStatus;
use crate::features::social::error::SocialError;
use crate::features::social::realtime::notify_social_changed;
use crate::features::social::support::{
    ensure_user_exists, friend_summaries, map_auth_error, parse_id, request_response,
    request_summary,
};
use crate::state::AppState;

pub(crate) use direct_messages::{
    list_dm_conversations, list_dm_messages, mark_dm_conversation_read, open_dm_conversation,
    send_dm_message,
};

const USER_SEARCH_LIMIT: u64 = 20;

/// Доступ voice-функции к личному диалогу без раскрытия доменной модели social.
#[derive(Debug, Clone)]
pub(crate) struct DirectMessageVoiceAccess {
    /// Идентификатор личного диалога.
    pub(crate) conversation_id: Uuid,
}

/// Проверяет доступ пользователя к голосовому звонку личного диалога.
pub(crate) async fn direct_message_voice_access(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<DirectMessageVoiceAccess, SocialError> {
    let conversation = state
        .social_store
        .conversation_by_id(conversation_id)
        .await
        .map_err(SocialError::Internal)?
        .ok_or_else(|| SocialError::NotFound("Диалог не найден.".to_owned()))?;
    if conversation.user_low_id != *user_id && conversation.user_high_id != *user_id {
        tracing::warn!(
            conversation_id = %conversation.id,
            user_id = %user_id,
            "rejected direct message voice access for non-participant"
        );
        return Err(SocialError::NotFound("Диалог не найден.".to_owned()));
    }

    let friend_user_id = if conversation.user_low_id == *user_id {
        conversation.user_high_id
    } else {
        conversation.user_low_id
    };
    ensure_accepted_friendship(state, user_id, &friend_user_id).await?;
    Ok(DirectMessageVoiceAccess {
        conversation_id: conversation.id,
    })
}

/// Возвращает доступные пользователю личные диалоги для списка активных voice-звонков.
pub(crate) async fn direct_message_voice_accesses_for_user(
    state: &AppState,
    user_id: &Uuid,
) -> Result<Vec<DirectMessageVoiceAccess>, SocialError> {
    let conversations = state
        .social_store
        .conversations_for_user(user_id)
        .await
        .map_err(SocialError::Internal)?;
    let mut access = Vec::new();
    for conversation in conversations {
        let friend_user_id = if conversation.user_low_id == *user_id {
            conversation.user_high_id
        } else {
            conversation.user_low_id
        };
        if accepted_friendship_exists(state, user_id, &friend_user_id).await? {
            access.push(DirectMessageVoiceAccess {
                conversation_id: conversation.id,
            });
        }
    }
    Ok(access)
}

/// Возвращает участников личного диалога для fanout voice-событий.
pub(crate) async fn direct_message_voice_user_ids(
    state: &AppState,
    conversation_id: &Uuid,
) -> Result<Vec<Uuid>, SocialError> {
    let Some(conversation) = state
        .social_store
        .conversation_by_id(conversation_id)
        .await
        .map_err(SocialError::Internal)?
    else {
        return Ok(Vec::new());
    };
    if accepted_friendship_exists(state, &conversation.user_low_id, &conversation.user_high_id)
        .await?
    {
        Ok(vec![conversation.user_low_id, conversation.user_high_id])
    } else {
        Ok(Vec::new())
    }
}

/// Ищет пользователей по никнейму через auth-хранилище.
pub(crate) async fn search_users(
    state: &AppState,
    access_token: &str,
    query: Option<String>,
) -> Result<SearchUsersResponse, SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let query = query.unwrap_or_default();
    let query = query.trim();
    if query.len() < 2 {
        return Ok(SearchUsersResponse { users: Vec::new() });
    }

    let users = state
        .auth_store
        .search_users_by_nickname(query, USER_SEARCH_LIMIT)
        .await
        .map_err(SocialError::Internal)?;
    let mut results = Vec::new();
    for user in users.into_iter().filter(|user| user.id != current_user.id) {
        let relation = relation_status(state, &current_user.id, &user.id).await?;
        let user = auth_user(state, &user);
        results.push(UserSearchResult {
            id: user.id,
            nickname: user.nickname,
            avatar_url: user.avatar_url,
            relation,
        });
    }

    Ok(SearchUsersResponse { users: results })
}

/// Возвращает друзей текущего пользователя.
pub(crate) async fn list_friends(
    state: &AppState,
    access_token: &str,
) -> Result<ListFriendsResponse, SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let friendships = state
        .social_store
        .friendships_for_user(&current_user.id, FriendshipStatus::Accepted)
        .await
        .map_err(SocialError::Internal)?;
    Ok(ListFriendsResponse {
        friends: friend_summaries(state, &current_user.id, friendships).await?,
    })
}

/// Возвращает входящие заявки.
pub(crate) async fn list_incoming_requests(
    state: &AppState,
    access_token: &str,
) -> Result<ListFriendRequestsResponse, SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let requests = state
        .social_store
        .incoming_requests(&current_user.id)
        .await
        .map_err(SocialError::Internal)?;
    request_response(state, requests).await
}

/// Возвращает исходящие заявки.
pub(crate) async fn list_outgoing_requests(
    state: &AppState,
    access_token: &str,
) -> Result<ListFriendRequestsResponse, SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let requests = state
        .social_store
        .outgoing_requests(&current_user.id)
        .await
        .map_err(SocialError::Internal)?;
    request_response(state, requests).await
}

/// Отправляет заявку в друзья.
pub(crate) async fn send_friend_request(
    state: &AppState,
    access_token: &str,
    request: SendFriendRequestRequest,
) -> Result<SendFriendRequestResponse, SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let recipient_user_id = parse_id(&request.recipient_user_id, "Пользователь не найден.")?;
    if recipient_user_id == current_user.id {
        return Err(SocialError::BadRequest(
            "Нельзя отправить заявку самому себе.".to_owned(),
        ));
    }
    ensure_user_exists(state, &recipient_user_id).await?;

    if let Some(existing) = state
        .social_store
        .friendship_between(&current_user.id, &recipient_user_id)
        .await
        .map_err(SocialError::Internal)?
    {
        match existing.status {
            FriendshipStatus::Pending => {
                return Err(SocialError::Conflict(
                    "Заявка уже ожидает ответ.".to_owned(),
                ));
            }
            FriendshipStatus::Accepted => {
                return Err(SocialError::Conflict(
                    "Пользователь уже в друзьях.".to_owned(),
                ));
            }
            FriendshipStatus::Declined | FriendshipStatus::Cancelled => {}
        }
    }

    tracing::info!(
        requester_user_id = %current_user.id,
        recipient_user_id = %recipient_user_id,
        "sending friend request"
    );
    let friendship = state
        .social_store
        .upsert_friend_request(&current_user.id, &recipient_user_id, chrono::Utc::now())
        .await
        .map_err(SocialError::Internal)?;
    notify_social_changed(
        state,
        &[current_user.id, recipient_user_id],
        SocialChangeReason::Friends,
        None,
    )
    .await;
    Ok(SendFriendRequestResponse {
        request: request_summary(state, friendship).await?,
    })
}

/// Принимает входящую заявку.
pub(crate) async fn accept_friend_request(
    state: &AppState,
    access_token: &str,
    request_id: String,
) -> Result<SendFriendRequestResponse, SocialError> {
    change_request_status(
        state,
        access_token,
        request_id,
        FriendshipStatus::Accepted,
        RequestActor::Recipient,
    )
    .await
}

/// Отклоняет входящую заявку.
pub(crate) async fn decline_friend_request(
    state: &AppState,
    access_token: &str,
    request_id: String,
) -> Result<SendFriendRequestResponse, SocialError> {
    change_request_status(
        state,
        access_token,
        request_id,
        FriendshipStatus::Declined,
        RequestActor::Recipient,
    )
    .await
}

/// Отменяет исходящую заявку.
pub(crate) async fn cancel_friend_request(
    state: &AppState,
    access_token: &str,
    request_id: String,
) -> Result<SendFriendRequestResponse, SocialError> {
    change_request_status(
        state,
        access_token,
        request_id,
        FriendshipStatus::Cancelled,
        RequestActor::Requester,
    )
    .await
}

/// Удаляет друга.
pub(crate) async fn delete_friend(
    state: &AppState,
    access_token: &str,
    friend_user_id: String,
) -> Result<(), SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let friend_user_id = parse_id(&friend_user_id, "Пользователь не найден.")?;
    let friendship = state
        .social_store
        .friendship_between(&current_user.id, &friend_user_id)
        .await
        .map_err(SocialError::Internal)?
        .ok_or_else(|| SocialError::NotFound("Дружба не найдена.".to_owned()))?;
    if friendship.status != FriendshipStatus::Accepted {
        return Err(SocialError::NotFound("Дружба не найдена.".to_owned()));
    }

    state
        .social_store
        .update_friendship_status(
            &friendship.id,
            FriendshipStatus::Cancelled,
            chrono::Utc::now(),
        )
        .await
        .map_err(SocialError::Internal)?;
    notify_social_changed(
        state,
        &[current_user.id, friend_user_id],
        SocialChangeReason::Friends,
        None,
    )
    .await;
    tracing::info!(user_id = %current_user.id, friend_user_id = %friend_user_id, "removed friend");
    Ok(())
}

#[derive(Clone, Copy)]
enum RequestActor {
    Requester,
    Recipient,
}

async fn change_request_status(
    state: &AppState,
    access_token: &str,
    request_id: String,
    next_status: FriendshipStatus,
    actor: RequestActor,
) -> Result<SendFriendRequestResponse, SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let request_id = parse_id(&request_id, "Заявка не найдена.")?;
    let friendship = state
        .social_store
        .friendship_by_id(&request_id)
        .await
        .map_err(SocialError::Internal)?
        .ok_or_else(|| SocialError::NotFound("Заявка не найдена.".to_owned()))?;
    if friendship.status != FriendshipStatus::Pending {
        return Err(SocialError::BadRequest(
            "Эта заявка уже обработана.".to_owned(),
        ));
    }
    let allowed = match actor {
        RequestActor::Requester => friendship.requester_user_id == current_user.id,
        RequestActor::Recipient => friendship.recipient_user_id == current_user.id,
    };
    if !allowed {
        return Err(SocialError::NotFound("Заявка не найдена.".to_owned()));
    }

    let updated = state
        .social_store
        .update_friendship_status(&friendship.id, next_status, chrono::Utc::now())
        .await
        .map_err(SocialError::Internal)?
        .ok_or_else(|| SocialError::NotFound("Заявка не найдена.".to_owned()))?;
    notify_social_changed(
        state,
        &[friendship.requester_user_id, friendship.recipient_user_id],
        SocialChangeReason::Friends,
        None,
    )
    .await;
    Ok(SendFriendRequestResponse {
        request: request_summary(state, updated).await?,
    })
}

async fn relation_status(
    state: &AppState,
    current_user_id: &Uuid,
    user_id: &Uuid,
) -> Result<Option<UserRelationStatus>, SocialError> {
    let Some(friendship) = state
        .social_store
        .friendship_between(current_user_id, user_id)
        .await
        .map_err(SocialError::Internal)?
    else {
        return Ok(None);
    };

    Ok(match friendship.status {
        FriendshipStatus::Accepted => Some(UserRelationStatus::Friends),
        FriendshipStatus::Pending if friendship.requester_user_id == *current_user_id => {
            Some(UserRelationStatus::PendingOutgoing)
        }
        FriendshipStatus::Pending => Some(UserRelationStatus::PendingIncoming),
        FriendshipStatus::Declined | FriendshipStatus::Cancelled => None,
    })
}

async fn ensure_accepted_friendship(
    state: &AppState,
    current_user_id: &Uuid,
    friend_user_id: &Uuid,
) -> Result<(), SocialError> {
    if accepted_friendship_exists(state, current_user_id, friend_user_id).await? {
        Ok(())
    } else {
        tracing::warn!(
            user_id = %current_user_id,
            friend_user_id = %friend_user_id,
            "rejected direct message voice access for non-friends"
        );
        Err(SocialError::Unauthorized(
            "Звонить можно только друзьям.".to_owned(),
        ))
    }
}

async fn accepted_friendship_exists(
    state: &AppState,
    left_user_id: &Uuid,
    right_user_id: &Uuid,
) -> Result<bool, SocialError> {
    let friendship = state
        .social_store
        .friendship_between(left_user_id, right_user_id)
        .await
        .map_err(SocialError::Internal)?;
    Ok(friendship.is_some_and(|friendship| friendship.status == FriendshipStatus::Accepted))
}

#[cfg(test)]
mod tests;
