//! In-memory-выборка страницы друзей.

use std::cmp::Ordering;

use uuid::Uuid;

use crate::features::social::domain::{FriendListCursor, FriendListEntry, FriendshipStatus};

use super::{FriendListPage, InMemorySocialStore};

pub(super) fn friend_list_page(
    store: &InMemorySocialStore,
    user_id: &Uuid,
    cursor: Option<&FriendListCursor>,
    limit: usize,
) -> anyhow::Result<FriendListPage> {
    let friendships = store
        .friendships
        .lock()
        .map_err(|_| poisoned())?
        .iter()
        .filter(|row| {
            row.status == FriendshipStatus::Accepted
                && (row.user_low_id == *user_id || row.user_high_id == *user_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let conversations = store.conversations.lock().map_err(|_| poisoned())?.clone();
    let messages = store.messages.lock().map_err(|_| poisoned())?.clone();
    let member_states = store.member_states.lock().map_err(|_| poisoned())?.clone();

    let mut entries = friendships
        .into_iter()
        .map(|friendship| {
            let friend_user_id = if friendship.user_low_id == *user_id {
                friendship.user_high_id
            } else {
                friendship.user_low_id
            };
            let conversation = conversations.iter().find(|row| {
                row.user_low_id == friendship.user_low_id
                    && row.user_high_id == friendship.user_high_id
            });
            let last_message = conversation.and_then(|conversation| {
                messages
                    .iter()
                    .filter(|row| {
                        row.conversation_id == conversation.id && row.deleted_at.is_none()
                    })
                    .max_by_key(|row| (row.seq, row.id))
                    .cloned()
            });
            let unread_count = conversation
                .and_then(|conversation| {
                    member_states.iter().find(|row| {
                        row.conversation_id == conversation.id && row.user_id == *user_id
                    })
                })
                .map_or(0, |state| state.unread_count);
            FriendListEntry {
                friendship,
                friend_user_id,
                unread_count,
                last_message,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(compare_friend_entries);
    if let Some(cursor) = cursor {
        entries.retain(|entry| friend_entry_is_after(entry, cursor));
    }
    let has_more = entries.len() > limit;
    entries.truncate(limit);
    Ok(FriendListPage { entries, has_more })
}

fn compare_friend_entries(left: &FriendListEntry, right: &FriendListEntry) -> Ordering {
    match (&left.last_message, &right.last_message) {
        (Some(left_message), Some(right_message)) => right_message
            .created_at
            .cmp(&left_message.created_at)
            .then_with(|| left.friend_user_id.cmp(&right.friend_user_id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.friend_user_id.cmp(&right.friend_user_id),
    }
}

fn friend_entry_is_after(entry: &FriendListEntry, cursor: &FriendListCursor) -> bool {
    match (entry.last_message.as_ref(), cursor.last_message_created_at) {
        (Some(message), Some(cursor_created_at)) => {
            message.created_at < cursor_created_at
                || (message.created_at == cursor_created_at
                    && entry.friend_user_id > cursor.friend_user_id)
        }
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (None, None) => entry.friend_user_id > cursor.friend_user_id,
    }
}

fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory social store lock poisoned")
}

#[cfg(test)]
mod tests;
