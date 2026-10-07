//! Создание и изменение системных строк `users` внутри серверных транзакций.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use uuid::Uuid;

use crate::features::auth::infrastructure::entities::users;

/// Имя и аватар системного автора для настроек сервера.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Profile {
    /// Отображаемое имя, редактируемое владельцем сервера.
    pub(crate) nickname: String,
    /// Изображение, совпадающее с аватаром сервера.
    pub(crate) avatar_image_id: Option<Uuid>,
}

/// Создаёт системный аккаунт в транзакции, принадлежащей bootstrap сервера.
pub(crate) async fn insert(
    transaction: &DatabaseTransaction,
    id: Uuid,
    nickname: String,
    avatar_image_id: Option<Uuid>,
    now: DateTime<Utc>,
) -> Result<(), DbErr> {
    users::ActiveModel {
        id: Set(id),
        nickname: Set(nickname),
        kind: Set("system".to_owned()),
        email: Set(None),
        email_normalized: Set(None),
        password_hash: Set(None),
        avatar_image_id: Set(avatar_image_id),
        registered_at: Set(now),
        nickname_updated_at: Set(now),
        deletion_requested_at: Set(None),
        deletion_restore_until: Set(None),
        deletion_token_hash: Set(None),
        deletion_finalized_at: Set(None),
        updated_at: Set(now),
    }
    .insert(transaction)
    .await?;

    Ok(())
}

/// Меняет отображаемое имя системного аккаунта, только если UUID действительно имеет kind system.
pub(crate) async fn update_nickname(
    transaction: &DatabaseTransaction,
    id: Uuid,
    nickname: String,
    now: DateTime<Utc>,
) -> Result<bool, DbErr> {
    let Some(account) = users::Entity::find_by_id(id).one(transaction).await? else {
        return Ok(false);
    };
    if account.kind != "system" {
        return Ok(false);
    }

    let mut account = account.into_active_model();
    account.nickname = Set(nickname);
    account.nickname_updated_at = Set(now);
    account.updated_at = Set(now);
    account.update(transaction).await?;

    Ok(true)
}

/// Обновляет аватар системного аккаунта, только если UUID действительно имеет kind system.
pub(crate) async fn update_avatar(
    transaction: &DatabaseTransaction,
    id: Uuid,
    avatar_image_id: Uuid,
    now: DateTime<Utc>,
) -> Result<bool, DbErr> {
    let Some(account) = users::Entity::find_by_id(id).one(transaction).await? else {
        return Ok(false);
    };
    if account.kind != "system" {
        return Ok(false);
    }

    let mut account = account.into_active_model();
    account.avatar_image_id = Set(Some(avatar_image_id));
    account.updated_at = Set(now);
    account.update(transaction).await?;

    Ok(true)
}

/// Возвращает профиль только для системной строки с заданным UUID.
pub(crate) async fn find_profile<C: ConnectionTrait>(
    database: &C,
    id: Uuid,
) -> Result<Option<Profile>, DbErr> {
    Ok(users::Entity::find_by_id(id)
        .filter(users::Column::Kind.eq("system"))
        .one(database)
        .await?
        .map(|account| Profile {
            nickname: account.nickname,
            avatar_image_id: account.avatar_image_id,
        }))
}

/// Удаляет системную строку внутри транзакции удаления сервера.
pub(crate) async fn delete(transaction: &DatabaseTransaction, id: Uuid) -> Result<bool, DbErr> {
    let result = users::Entity::delete_many()
        .filter(users::Column::Id.eq(id))
        .filter(users::Column::Kind.eq("system"))
        .exec(transaction)
        .await?;

    Ok(result.rows_affected > 0)
}
