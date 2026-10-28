//! Локальная реализация объединённой социальной выборки для тестов и разработки.

use std::cmp::Ordering;
use std::collections::HashSet;

use uuid::Uuid;

use crate::features::auth::infrastructure::AuthStore;
use crate::features::social::domain::{FriendshipStatus, SocialListCursor, SocialListEntry};

use super::{InMemorySocialStore, SocialListPage};

pub(super) async fn social_list_page(
    store: &InMemorySocialStore,
    user_id: &Uuid,
    normalized_query: &str,
    cursor: Option<&SocialListCursor>,
    limit: usize,
    users: &dyn AuthStore,
) -> anyhow::Result<SocialListPage> {
    let friendships = store.friendships.lock().map_err(|_| poisoned())?.clone();
    let conversations = store
        .conversations
        .lock()
        .map_err(|_| poisoned())?
        .iter()
        .filter(|conversation| {
            conversation.user_low_id == *user_id || conversation.user_high_id == *user_id
        })
        .cloned()
        .collect::<Vec<_>>();
    let messages = store.messages.lock().map_err(|_| poisoned())?.clone();
    let member_states = store.member_states.lock().map_err(|_| poisoned())?.clone();

    let mut peer_ids = HashSet::new();
    for friendship in friendships.iter().filter(|friendship| {
        friendship.status == FriendshipStatus::Accepted
            && (friendship.user_low_id == *user_id || friendship.user_high_id == *user_id)
    }) {
        peer_ids.insert(other_id(
            friendship.user_low_id,
            friendship.user_high_id,
            *user_id,
        ));
    }
    for conversation in &conversations {
        peer_ids.insert(other_id(
            conversation.user_low_id,
            conversation.user_high_id,
            *user_id,
        ));
    }
    let ids = peer_ids.into_iter().collect::<Vec<_>>();
    let profiles = users.find_users_by_ids(&ids).await?;
    let mut entries = profiles
        .into_iter()
        .filter_map(|profile| {
            let nickname_key = profile.nickname.trim().to_lowercase();
            if !nickname_key.contains(normalized_query) {
                return None;
            }
            let conversation = conversations
                .iter()
                .find(|conversation| {
                    conversation.user_low_id == profile.id
                        || conversation.user_high_id == profile.id
                })
                .cloned();
            let last_message = conversation.as_ref().and_then(|conversation| {
                messages
                    .iter()
                    .filter(|message| {
                        message.conversation_id == conversation.id && message.deleted_at.is_none()
                    })
                    .max_by_key(|message| (message.seq, message.id))
                    .cloned()
            });
            let read_state = conversation.as_ref().and_then(|conversation| {
                member_states
                    .iter()
                    .find(|state| {
                        state.conversation_id == conversation.id && state.user_id == *user_id
                    })
                    .cloned()
            });
            Some(SocialListEntry {
                user_id: profile.id,
                nickname_key,
                conversation,
                read_state,
                last_message,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(compare_entries);
    if let Some(cursor) = cursor {
        entries.retain(|entry| is_after(entry, cursor));
    }
    let has_more = entries.len() > limit;
    entries.truncate(limit);
    Ok(SocialListPage { entries, has_more })
}

fn other_id(low_id: Uuid, high_id: Uuid, user_id: Uuid) -> Uuid {
    if low_id == user_id { high_id } else { low_id }
}

fn compare_names(left_name: &str, left_id: Uuid, right_name: &str, right_id: Uuid) -> Ordering {
    left_name
        .cmp(right_name)
        .then_with(|| left_id.cmp(&right_id))
}

fn compare_entries(left: &SocialListEntry, right: &SocialListEntry) -> Ordering {
    match (&left.last_message, &right.last_message) {
        (Some(left_message), Some(right_message)) => right_message
            .created_at
            .cmp(&left_message.created_at)
            .then_with(|| {
                compare_names(
                    &left.nickname_key,
                    left.user_id,
                    &right.nickname_key,
                    right.user_id,
                )
            }),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => compare_names(
            &left.nickname_key,
            left.user_id,
            &right.nickname_key,
            right.user_id,
        ),
    }
}

fn is_after(entry: &SocialListEntry, cursor: &SocialListCursor) -> bool {
    let later_name = compare_names(
        &entry.nickname_key,
        entry.user_id,
        &cursor.nickname_key,
        cursor.user_id,
    ) == Ordering::Greater;
    match (entry.last_message.as_ref(), cursor.last_message_created_at) {
        (Some(message), Some(created_at)) => {
            message.created_at < created_at || (message.created_at == created_at && later_name)
        }
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (None, None) => later_name,
    }
}

fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory social store lock poisoned")
}
