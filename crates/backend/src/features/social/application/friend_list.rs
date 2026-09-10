//! Выдача отсортированных страниц друзей.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use cheenhub_contracts::rest::{ListFriendsQuery, ListFriendsResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::auth::application::require_current_user;
use crate::features::social::domain::{FriendListCursor, FriendListEntry};
use crate::features::social::error::SocialError;
use crate::features::social::support::{friend_summaries, map_auth_error};
use crate::state::AppState;

const DEFAULT_FRIENDS_LIMIT: usize = 50;
const MAX_FRIENDS_LIMIT: usize = 100;

#[derive(Debug, Serialize, Deserialize)]
struct FriendsCursor {
    last_message_created_at: Option<DateTime<Utc>>,
    friend_user_id: Uuid,
}

/// Возвращает друзей текущего пользователя в порядке активности личных сообщений.
pub(crate) async fn list_friends(
    state: &AppState,
    access_token: &str,
    query: ListFriendsQuery,
) -> Result<ListFriendsResponse, SocialError> {
    let (current_user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let limit = friends_limit(query.limit)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(decode_friends_cursor)
        .transpose()?;
    let store_cursor = cursor.as_ref().map(|cursor| FriendListCursor {
        last_message_created_at: cursor.last_message_created_at,
        friend_user_id: cursor.friend_user_id,
    });
    let page = state
        .social_store
        .friend_list_page(&current_user.id, store_cursor.as_ref(), limit)
        .await
        .map_err(SocialError::Internal)?;
    let next_cursor = if page.has_more {
        page.entries.last().map(encode_friends_cursor).transpose()?
    } else {
        None
    };
    let friends = friend_summaries(state, page.entries).await?;
    tracing::debug!(
        user_id = %current_user.id,
        returned_count = friends.len(),
        has_more = page.has_more,
        "listed friends page"
    );
    Ok(ListFriendsResponse {
        friends,
        next_cursor,
        has_more: page.has_more,
    })
}

fn friends_limit(limit: Option<u32>) -> Result<usize, SocialError> {
    let limit = limit.map_or(DEFAULT_FRIENDS_LIMIT, |value| value as usize);
    if limit == 0 || limit > MAX_FRIENDS_LIMIT {
        tracing::warn!(limit, "rejected invalid friends page limit");
        return Err(SocialError::BadRequest(format!(
            "Параметр limit должен быть от 1 до {MAX_FRIENDS_LIMIT}."
        )));
    }
    Ok(limit)
}

fn encode_friends_cursor(entry: &FriendListEntry) -> Result<String, SocialError> {
    let cursor = FriendsCursor {
        last_message_created_at: entry
            .last_message
            .as_ref()
            .map(|message| message.created_at),
        friend_user_id: entry.friend_user_id,
    };
    serde_json::to_vec(&cursor)
        .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
        .map_err(|error| SocialError::Internal(error.into()))
}

fn decode_friends_cursor(value: &str) -> Result<FriendsCursor, SocialError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| invalid_friends_cursor())?;
    serde_json::from_slice(&bytes).map_err(|_| invalid_friends_cursor())
}

fn invalid_friends_cursor() -> SocialError {
    tracing::warn!("rejected invalid friends page cursor");
    SocialError::BadRequest("Курсор списка друзей недействителен.".to_owned())
}
