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
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::features::social::domain::{DmConversation, DmMessage, Friendship};

    #[test]
    fn latest_message_is_selected_by_sequence_then_identifier() {
        let current_user_id = uuid(1);
        let friend_user_id = uuid(2);
        let conversation_id = uuid(10);
        let store = store_with_friendships(&current_user_id, &[friend_user_id]);
        store
            .conversations
            .lock()
            .expect("conversations lock should work")
            .push(conversation(
                conversation_id,
                current_user_id,
                friend_user_id,
            ));
        store
            .messages
            .lock()
            .expect("messages lock should work")
            .extend([
                message(uuid(20), conversation_id, 1, timestamp(20)),
                message(uuid(21), conversation_id, 2, timestamp(10)),
            ]);

        let page =
            friend_list_page(&store, &current_user_id, None, 10).expect("friends page should load");

        assert_eq!(
            page.entries[0].last_message.as_ref().map(|row| row.seq),
            Some(2)
        );
    }

    #[test]
    fn keyset_paginates_friends_with_equal_message_timestamps() {
        let current_user_id = uuid(1);
        let friend_ids = [uuid(2), uuid(3), uuid(4)];
        let store = store_with_friendships(&current_user_id, &friend_ids);
        let created_at = timestamp(10);
        for (index, friend_user_id) in friend_ids.into_iter().enumerate() {
            let conversation_id = uuid(10 + index as u128);
            store
                .conversations
                .lock()
                .expect("conversations lock should work")
                .push(conversation(
                    conversation_id,
                    current_user_id,
                    friend_user_id,
                ));
            store
                .messages
                .lock()
                .expect("messages lock should work")
                .push(message(
                    uuid(20 + index as u128),
                    conversation_id,
                    1,
                    created_at,
                ));
        }

        assert_pages(&store, current_user_id, &friend_ids);
    }

    #[test]
    fn keyset_paginates_multiple_friends_without_messages() {
        let current_user_id = uuid(1);
        let friend_ids = [uuid(2), uuid(3), uuid(4)];
        let store = store_with_friendships(&current_user_id, &friend_ids);

        assert_pages(&store, current_user_id, &friend_ids);
    }

    fn assert_pages(
        store: &InMemorySocialStore,
        current_user_id: Uuid,
        expected_friend_ids: &[Uuid],
    ) {
        let mut cursor = None;
        let mut actual_friend_ids = Vec::new();
        loop {
            let page = friend_list_page(store, &current_user_id, cursor.as_ref(), 1)
                .expect("friends page should load");
            let entry = page.entries.first().expect("page should contain a friend");
            actual_friend_ids.push(entry.friend_user_id);
            if !page.has_more {
                break;
            }
            cursor = Some(FriendListCursor {
                last_message_created_at: entry
                    .last_message
                    .as_ref()
                    .map(|message| message.created_at),
                friend_user_id: entry.friend_user_id,
            });
        }
        assert_eq!(actual_friend_ids, expected_friend_ids);
    }

    fn store_with_friendships(
        current_user_id: &Uuid,
        friend_user_ids: &[Uuid],
    ) -> InMemorySocialStore {
        let store = InMemorySocialStore::default();
        store
            .friendships
            .lock()
            .expect("friendships lock should work")
            .extend(friend_user_ids.iter().map(|friend_user_id| Friendship {
                id: Uuid::new_v4(),
                requester_user_id: *current_user_id,
                recipient_user_id: *friend_user_id,
                user_low_id: *current_user_id,
                user_high_id: *friend_user_id,
                status: FriendshipStatus::Accepted,
                created_at: timestamp(1),
                updated_at: timestamp(1),
            }));
        store
    }

    fn conversation(id: Uuid, current_user_id: Uuid, friend_user_id: Uuid) -> DmConversation {
        DmConversation {
            id,
            user_low_id: current_user_id,
            user_high_id: friend_user_id,
            updated_at: timestamp(1),
        }
    }

    fn message(
        id: Uuid,
        conversation_id: Uuid,
        seq: i64,
        created_at: chrono::DateTime<Utc>,
    ) -> DmMessage {
        DmMessage {
            id,
            conversation_id,
            seq,
            sender_user_id: uuid(2),
            body: format!("Сообщение {seq}"),
            image_id: None,
            created_at,
            updated_at: created_at,
            deleted_at: None,
        }
    }

    fn timestamp(seconds: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(seconds, 0)
            .single()
            .expect("timestamp should be valid")
    }

    fn uuid(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }
}
