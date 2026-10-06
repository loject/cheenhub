//! Postgres-операции над записями дружбы.
//!
//! Методы трейта хранилища разнесены по файлам по сущности: этот модуль отвечает
//! только за связи дружбы, а диалоги, сообщения и read-state остаются в
//! `postgres.rs`. Разделение не меняет запросы, но удерживает каждый файл в
//! пределах размера, установленного для исходных файлов проекта.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::features::social::domain::{Friendship, FriendshipStatus, ordered_pair};
use crate::features::social::infrastructure::entities as friendships;

/// Находит запись дружбы для пары пользователей.
pub(super) async fn friendship_between(
    database: &DatabaseConnection,
    left_user_id: &Uuid,
    right_user_id: &Uuid,
) -> anyhow::Result<Option<Friendship>> {
    let (user_low_id, user_high_id) = ordered_pair(*left_user_id, *right_user_id);
    friendships::Entity::find()
        .filter(friendships::Column::UserLowId.eq(user_low_id))
        .filter(friendships::Column::UserHighId.eq(user_high_id))
        .one(database)
        .await?
        .map(try_friendship)
        .transpose()
}

/// Находит запись дружбы по идентификатору.
pub(super) async fn friendship_by_id(
    database: &DatabaseConnection,
    friendship_id: &Uuid,
) -> anyhow::Result<Option<Friendship>> {
    friendships::Entity::find_by_id(*friendship_id)
        .one(database)
        .await?
        .map(try_friendship)
        .transpose()
}

/// Находит записи дружбы пользователя сразу по набору собеседников.
pub(super) async fn friendships_with_user(
    database: &DatabaseConnection,
    user_id: &Uuid,
    other_user_ids: &[Uuid],
) -> anyhow::Result<Vec<Friendship>> {
    if other_user_ids.is_empty() {
        return Ok(Vec::new());
    }
    // Отношение пары хранится в упорядоченном виде, поэтому собеседник может
    // оказаться как в нижней, так и в верхней половине пары.
    rows_to_friendships(
        friendships::Entity::find()
            .filter(
                Condition::any()
                    .add(friendships::Column::UserLowId.eq(*user_id))
                    .add(friendships::Column::UserHighId.eq(*user_id)),
            )
            .filter(
                Condition::any()
                    .add(friendships::Column::UserLowId.is_in(other_user_ids.iter().copied()))
                    .add(friendships::Column::UserHighId.is_in(other_user_ids.iter().copied())),
            )
            .all(database)
            .await?,
    )
}
/// Создает новую заявку или переоткрывает существующую пару.
pub(super) async fn upsert_friend_request(
    database: &DatabaseConnection,
    requester_user_id: &Uuid,
    recipient_user_id: &Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<Friendship> {
    let (user_low_id, user_high_id) = ordered_pair(*requester_user_id, *recipient_user_id);
    if let Some(row) = friendships::Entity::find()
        .filter(friendships::Column::UserLowId.eq(user_low_id))
        .filter(friendships::Column::UserHighId.eq(user_high_id))
        .one(database)
        .await?
    {
        let mut active = row.into_active_model();
        active.requester_user_id = Set(*requester_user_id);
        active.recipient_user_id = Set(*recipient_user_id);
        active.status = Set(FriendshipStatus::Pending.as_str().to_owned());
        active.updated_at = Set(now);
        return try_friendship(active.update(database).await?);
    }

    try_friendship(
        friendships::ActiveModel {
            id: Set(Uuid::new_v4()),
            requester_user_id: Set(*requester_user_id),
            recipient_user_id: Set(*recipient_user_id),
            user_low_id: Set(user_low_id),
            user_high_id: Set(user_high_id),
            status: Set(FriendshipStatus::Pending.as_str().to_owned()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(database)
        .await?,
    )
}

/// Меняет статус записи дружбы.
pub(super) async fn update_friendship_status(
    database: &DatabaseConnection,
    friendship_id: &Uuid,
    status: FriendshipStatus,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<Friendship>> {
    let Some(row) = friendships::Entity::find_by_id(*friendship_id)
        .one(database)
        .await?
    else {
        return Ok(None);
    };
    let mut active = row.into_active_model();
    active.status = Set(status.as_str().to_owned());
    active.updated_at = Set(now);
    Ok(Some(try_friendship(active.update(database).await?)?))
}

/// Возвращает ожидающие заявки пользователя по указанной колонке роли.
///
/// Входящие и исходящие отличаются только колонкой, поэтому запрос строится один раз.
pub(super) async fn request_rows(
    database: &DatabaseConnection,
    user_column: friendships::Column,
    user_id: &Uuid,
) -> anyhow::Result<Vec<Friendship>> {
    rows_to_friendships(
        friendships::Entity::find()
            .filter(friendships::Column::Status.eq(FriendshipStatus::Pending.as_str()))
            .filter(user_column.eq(*user_id))
            .order_by_desc(friendships::Column::CreatedAt)
            .all(database)
            .await?,
    )
}

fn rows_to_friendships(rows: Vec<friendships::Model>) -> anyhow::Result<Vec<Friendship>> {
    rows.into_iter().map(try_friendship).collect()
}

fn try_friendship(row: friendships::Model) -> anyhow::Result<Friendship> {
    let status = FriendshipStatus::from_str(&row.status)
        .ok_or_else(|| anyhow::anyhow!("unknown friendship status {}", row.status))?;
    Ok(Friendship {
        id: row.id,
        requester_user_id: row.requester_user_id,
        recipient_user_id: row.recipient_user_id,
        user_low_id: row.user_low_id,
        user_high_id: row.user_high_id,
        status,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}
