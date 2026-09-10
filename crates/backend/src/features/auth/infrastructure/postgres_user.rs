//! Вспомогательные функции хранения пользователей для Postgres.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::features::auth::domain::{RegistrationLegalAcceptance, UserAccount};
use crate::features::auth::infrastructure::entities::{legal_acceptances, users};
use crate::features::auth::infrastructure::{InsertUserError, UserConflict};

/// Атомарно создаёт пользователя и журнал подтверждённых юридических документов.
pub(super) async fn insert_user(
    database: &DatabaseConnection,
    nickname: String,
    email: String,
    email_normalized: String,
    password_hash: Option<String>,
    legal_acceptance: RegistrationLegalAcceptance,
    now: DateTime<Utc>,
) -> Result<UserAccount, InsertUserError> {
    let user_id = Uuid::new_v4();
    let transaction = database.begin().await.map_err(InsertUserError::Database)?;
    let model = users::ActiveModel {
        id: Set(user_id),
        nickname: Set(nickname),
        email: Set(email),
        email_normalized: Set(email_normalized),
        password_hash: Set(password_hash),
        avatar_image_id: Set(None),
        registered_at: Set(now),
        nickname_updated_at: Set(now),
        accepted_terms_at: Set(now),
        deletion_requested_at: Set(None),
        deletion_restore_until: Set(None),
        deletion_token_hash: Set(None),
        deletion_finalized_at: Set(None),
        updated_at: Set(now),
    }
    .insert(&transaction)
    .await
    .map_err(map_insert_user_error)?;

    legal_acceptances::Entity::insert_many([
        legal_acceptances::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            document_kind: Set("terms".to_owned()),
            document_version: Set(legal_acceptance.terms_version),
            acceptance_source: Set(legal_acceptance.acceptance_source.clone()),
            accepted_at: Set(now),
        },
        legal_acceptances::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            document_kind: Set("privacy_policy".to_owned()),
            document_version: Set(legal_acceptance.privacy_policy_version),
            acceptance_source: Set(legal_acceptance.acceptance_source.clone()),
            accepted_at: Set(now),
        },
        legal_acceptances::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            document_kind: Set("personal_data_consent".to_owned()),
            document_version: Set(legal_acceptance.personal_data_consent_version),
            acceptance_source: Set(legal_acceptance.acceptance_source),
            accepted_at: Set(now),
        },
    ])
    .exec(&transaction)
    .await
    .map_err(InsertUserError::Database)?;
    transaction
        .commit()
        .await
        .map_err(InsertUserError::Database)?;

    Ok(model.into())
}

/// Сопоставляет ошибки вставки в базу с конфликтами полей пользователя.
pub(super) fn map_insert_user_error(error: sea_orm::DbErr) -> InsertUserError {
    let message = error.to_string();
    if message.contains("users_nickname_key") {
        return InsertUserError::Conflict(UserConflict::Nickname);
    }
    if message.contains("users_email_normalized_key") {
        return InsertUserError::Conflict(UserConflict::Email);
    }

    InsertUserError::Database(error)
}

fn escape_like_pattern(value: &str) -> String {
    value.chars().fold(String::new(), |mut escaped, ch| {
        match ch {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
        escaped
    })
}

/// Находит пользователя по нормализованному email.
pub(super) async fn find_user_by_email(
    database: &DatabaseConnection,
    email_normalized: &str,
) -> anyhow::Result<Option<UserAccount>> {
    Ok(users::Entity::find()
        .filter(users::Column::EmailNormalized.eq(email_normalized))
        .one(database)
        .await?
        .map(Into::into))
}

/// Находит пользователя по идентификатору.
pub(super) async fn find_user_by_id(
    database: &DatabaseConnection,
    user_id: &Uuid,
) -> anyhow::Result<Option<UserAccount>> {
    Ok(users::Entity::find_by_id(*user_id)
        .one(database)
        .await?
        .map(|mut user| {
            if user.deletion_requested_at.is_some() {
                user.nickname = "Удалённый пользователь".to_owned();
                user.avatar_image_id = None;
            }
            user.into()
        }))
}

/// Находит пользователей по набору идентификаторов.
pub(super) async fn find_users_by_ids(
    database: &DatabaseConnection,
    user_ids: &[Uuid],
) -> anyhow::Result<Vec<UserAccount>> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.iter().copied()))
        .all(database)
        .await?
        .into_iter()
        .map(|mut user| {
            if user.deletion_requested_at.is_some() {
                user.nickname = "Удалённый пользователь".to_owned();
                user.avatar_image_id = None;
            }
            user.into()
        })
        .collect())
}

/// Ищет пользователей по никнейму.
pub(super) async fn search_users_by_nickname(
    database: &DatabaseConnection,
    query: &str,
    limit: u64,
) -> anyhow::Result<Vec<UserAccount>> {
    let pattern = format!("%{}%", escape_like_pattern(query));
    Ok(users::Entity::find()
        .filter(users::Column::Nickname.like(pattern))
        .filter(super::postgres_deletion::active_users_filter())
        .order_by_asc(users::Column::Nickname)
        .limit(limit)
        .all(database)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Читает изображения аватаров выбранных пользователей.
pub(super) async fn avatar_image_ids_by_user_ids(
    database: &DatabaseConnection,
    user_ids: &[Uuid],
) -> anyhow::Result<HashMap<Uuid, Uuid>> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    Ok(users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids.iter().copied()))
        .filter(super::postgres_deletion::active_users_filter())
        .all(database)
        .await?
        .into_iter()
        .filter_map(|user| user.avatar_image_id.map(|image_id| (user.id, image_id)))
        .collect())
}
