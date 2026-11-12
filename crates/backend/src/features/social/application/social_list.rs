//! Единый realtime-список друзей и существующих личных собеседников.
//!
//! Прикладной сценарий проверяет пагинацию и собирает публичные DTO страницы.
//! Объединение, фильтрацию и keyset-порядок обеспечивает хранилище social.

use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use cheenhub_contracts::realtime::{ListSocialEntries, SocialEntries, SocialListEntrySummary};
use cheenhub_contracts::rest::{DmLastMessageSummary, UserRelationStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::auth::application::auth_user;
use crate::features::social::domain::{DmMessage, SocialListCursor, SocialListEntry};
use crate::features::social::error::SocialError;
use crate::features::social::presence::public_presence;
use crate::features::social::support::{conversation_summary_from_parts, default_member_state};
use crate::state::AppState;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;
const MAX_QUERY_CHARS: usize = 128;
const MAX_CURSOR_BYTES: usize = 4096;

#[derive(Debug, Serialize, Deserialize)]
struct WireCursor {
    normalized_query: String,
    last_message_created_at: Option<DateTime<Utc>>,
    nickname_key: String,
    user_id: Uuid,
}

/// Возвращает страницу участников списка от имени проверенной realtime-сессии.
///
/// Курсор относится только к нормализованному поисковому запросу. При изменении
/// социального графа или активности сообщений клиент начинает пагинацию заново.
pub(crate) async fn list_social_entries(
    state: &AppState,
    user_id: &Uuid,
    request: ListSocialEntries,
) -> Result<SocialEntries, SocialError> {
    let normalized_query = request.query.unwrap_or_default().trim().to_lowercase();
    if normalized_query.chars().count() > MAX_QUERY_CHARS {
        tracing::warn!(%user_id, "rejected overlong social list search");
        return Err(SocialError::BadRequest(
            "Поисковый запрос слишком длинный.".to_owned(),
        ));
    }
    let limit = request.limit.map_or(DEFAULT_LIMIT, |value| value as usize);
    if !(1..=MAX_LIMIT).contains(&limit) {
        tracing::warn!(%user_id, limit, "rejected invalid social list limit");
        return Err(SocialError::BadRequest(format!(
            "Параметр limit должен быть от 1 до {MAX_LIMIT}."
        )));
    }
    let wire_cursor = request.cursor.as_deref().map(decode_cursor).transpose()?;
    if wire_cursor
        .as_ref()
        .is_some_and(|cursor| cursor.normalized_query != normalized_query)
    {
        tracing::warn!(%user_id, "rejected social list cursor from another search");
        return Err(invalid_cursor());
    }
    let cursor = wire_cursor.as_ref().map(|cursor| SocialListCursor {
        last_message_created_at: cursor.last_message_created_at,
        nickname_key: cursor.nickname_key.clone(),
        user_id: cursor.user_id,
    });
    let page = state
        .social_store
        .social_list_page(
            user_id,
            &normalized_query,
            cursor.as_ref(),
            limit,
            state.auth_store.as_ref(),
        )
        .await
        .map_err(SocialError::Internal)?;
    let next_cursor = if page.has_more {
        page.entries
            .last()
            .map(|entry| encode_cursor(entry, &normalized_query))
            .transpose()?
    } else {
        None
    };

    let user_ids = page
        .entries
        .iter()
        .map(|entry| entry.user_id)
        .collect::<Vec<_>>();
    let users = state
        .auth_store
        .find_users_by_ids(&user_ids)
        .await
        .map_err(SocialError::Internal)?
        .into_iter()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();
    let relations = super::relation_statuses(state, user_id, &user_ids).await?;
    let block_statuses = super::block_statuses_for_peers(state, user_id, &user_ids).await?;
    let mut entries = Vec::with_capacity(page.entries.len());
    for entry in page.entries {
        let account = users.get(&entry.user_id).ok_or_else(|| {
            SocialError::NotFound("Пользователь больше не существует.".to_owned())
        })?;
        let user = auth_user(state, account);
        let relation = relations.get(&entry.user_id).copied();
        let presence = if relation == Some(UserRelationStatus::Friends) {
            let activity = state
                .realtime_hub
                .session_activity_for_user(entry.user_id)
                .await;
            Some(public_presence(activity))
        } else {
            None
        };
        let conversation = entry.conversation.map(|conversation| {
            let read_state = entry
                .read_state
                .unwrap_or_else(|| default_member_state(&conversation, user_id));
            conversation_summary_from_parts(
                user.clone(),
                conversation,
                read_state,
                relation,
                block_statuses
                    .get(&entry.user_id)
                    .copied()
                    .unwrap_or_default(),
            )
        });
        entries.push(SocialListEntrySummary {
            user_id: user.id,
            nickname: user.nickname,
            avatar_url: user.avatar_url,
            relation,
            presence,
            conversation,
            last_message: entry.last_message.map(last_message_summary),
        });
    }
    tracing::debug!(%user_id, returned_count = entries.len(), has_more = page.has_more, has_query = !normalized_query.is_empty(), "listed unified social page");
    Ok(SocialEntries {
        entries,
        next_cursor,
        has_more: page.has_more,
    })
}

fn last_message_summary(message: DmMessage) -> DmLastMessageSummary {
    DmLastMessageSummary {
        id: message.id.to_string(),
        sender_user_id: message.sender_user_id.to_string(),
        body: message.body,
        has_image: message.image_id.is_some(),
        created_at: message.created_at.to_rfc3339(),
    }
}

fn encode_cursor(entry: &SocialListEntry, normalized_query: &str) -> Result<String, SocialError> {
    let cursor = WireCursor {
        normalized_query: normalized_query.to_owned(),
        last_message_created_at: entry
            .last_message
            .as_ref()
            .map(|message| message.created_at),
        nickname_key: entry.nickname_key.clone(),
        user_id: entry.user_id,
    };
    serde_json::to_vec(&cursor)
        .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
        .map_err(|error| SocialError::Internal(error.into()))
}

fn decode_cursor(value: &str) -> Result<WireCursor, SocialError> {
    if value.len() > MAX_CURSOR_BYTES {
        return Err(invalid_cursor());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| invalid_cursor())?;
    serde_json::from_slice(&bytes).map_err(|_| invalid_cursor())
}

fn invalid_cursor() -> SocialError {
    tracing::warn!("rejected invalid social list cursor");
    SocialError::BadRequest("Курсор общего списка недействителен.".to_owned())
}
