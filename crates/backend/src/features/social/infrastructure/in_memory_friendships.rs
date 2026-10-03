//! In-memory-операции над записями дружбы.
//!
//! Методы трейта хранилища разнесены по файлам по сущности: этот модуль отвечает
//! только за связи дружбы, а диалоги и сообщения остаются в `in_memory.rs`. Такой
//! разнос не меняет поведение in-memory-хранилища, но не даёт одному файлу
//! разрастаться за счёт несвязанных сущностей.

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::InMemorySocialStore;
use crate::features::social::domain::{Friendship, FriendshipStatus, ordered_pair};

/// Находит запись дружбы для пары пользователей.
pub(super) fn friendship_between(
    store: &InMemorySocialStore,
    left_user_id: &Uuid,
    right_user_id: &Uuid,
) -> anyhow::Result<Option<Friendship>> {
    let (user_low_id, user_high_id) = ordered_pair(*left_user_id, *right_user_id);
    Ok(store
        .friendships
        .lock()
        .map_err(|_| poisoned())?
        .iter()
        .find(|row| row.user_low_id == user_low_id && row.user_high_id == user_high_id)
        .cloned())
}

/// Находит запись дружбы по идентификатору.
pub(super) fn friendship_by_id(
    store: &InMemorySocialStore,
    friendship_id: &Uuid,
) -> anyhow::Result<Option<Friendship>> {
    Ok(store
        .friendships
        .lock()
        .map_err(|_| poisoned())?
        .iter()
        .find(|row| row.id == *friendship_id)
        .cloned())
}

/// Находит записи дружбы пользователя сразу по набору собеседников.
pub(super) fn friendships_with_user(
    store: &InMemorySocialStore,
    user_id: &Uuid,
    other_user_ids: &[Uuid],
) -> anyhow::Result<Vec<Friendship>> {
    if other_user_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(store
        .friendships
        .lock()
        .map_err(|_| poisoned())?
        .iter()
        .filter(|row| {
            friendship_peer(row, user_id).is_some_and(|peer| other_user_ids.contains(&peer))
        })
        .cloned()
        .collect())
}

/// Создает новую заявку или переоткрывает существующую пару.
pub(super) fn upsert_friend_request(
    store: &InMemorySocialStore,
    requester_user_id: &Uuid,
    recipient_user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<Friendship> {
    let (user_low_id, user_high_id) = ordered_pair(*requester_user_id, *recipient_user_id);
    let mut friendships = store.friendships.lock().map_err(|_| poisoned())?;
    if let Some(row) = friendships
        .iter_mut()
        .find(|row| row.user_low_id == user_low_id && row.user_high_id == user_high_id)
    {
        row.requester_user_id = *requester_user_id;
        row.recipient_user_id = *recipient_user_id;
        row.status = FriendshipStatus::Pending;
        row.updated_at = now;
        return Ok(row.clone());
    }

    let friendship = Friendship {
        id: Uuid::new_v4(),
        requester_user_id: *requester_user_id,
        recipient_user_id: *recipient_user_id,
        user_low_id,
        user_high_id,
        status: FriendshipStatus::Pending,
        created_at: now,
        updated_at: now,
    };
    friendships.push(friendship.clone());
    Ok(friendship)
}

/// Меняет статус записи дружбы.
pub(super) fn update_friendship_status(
    store: &InMemorySocialStore,
    friendship_id: &Uuid,
    status: FriendshipStatus,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<Friendship>> {
    let mut friendships = store.friendships.lock().map_err(|_| poisoned())?;
    let Some(row) = friendships.iter_mut().find(|row| row.id == *friendship_id) else {
        return Ok(None);
    };
    row.status = status;
    row.updated_at = now;
    Ok(Some(row.clone()))
}

/// Возвращает ожидающие входящие заявки пользователя.
pub(super) fn incoming_requests(
    store: &InMemorySocialStore,
    user_id: &Uuid,
) -> anyhow::Result<Vec<Friendship>> {
    Ok(store
        .friendships
        .lock()
        .map_err(|_| poisoned())?
        .iter()
        .filter(|row| row.status == FriendshipStatus::Pending && row.recipient_user_id == *user_id)
        .cloned()
        .collect())
}

/// Возвращает ожидающие исходящие заявки пользователя.
pub(super) fn outgoing_requests(
    store: &InMemorySocialStore,
    user_id: &Uuid,
) -> anyhow::Result<Vec<Friendship>> {
    Ok(store
        .friendships
        .lock()
        .map_err(|_| poisoned())?
        .iter()
        .filter(|row| row.status == FriendshipStatus::Pending && row.requester_user_id == *user_id)
        .cloned()
        .collect())
}

/// Возвращает собеседника по записи дружбы, если пользователь участвует в паре.
fn friendship_peer(friendship: &Friendship, user_id: &Uuid) -> Option<Uuid> {
    if friendship.user_low_id == *user_id {
        Some(friendship.user_high_id)
    } else if friendship.user_high_id == *user_id {
        Some(friendship.user_low_id)
    } else {
        None
    }
}

fn poisoned() -> anyhow::Error {
    anyhow!("in-memory social store lock poisoned")
}
