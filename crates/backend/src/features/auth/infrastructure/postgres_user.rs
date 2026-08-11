//! Вспомогательные функции хранения пользователей для Postgres.

use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, TransactionTrait};
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
