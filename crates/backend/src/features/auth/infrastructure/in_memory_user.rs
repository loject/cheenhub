//! Хранение и поиск пользователей в памяти для тестов и разработки.

use super::in_memory::{
    model::{InMemoryState, InMemoryUser},
    poisoned,
};
use super::{InsertUserError, UserConflict};
use crate::features::auth::domain::{RegistrationLegalAcceptance, UserAccount};
use chrono::{DateTime, Utc};
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

/// Создаёт пользователя и сохраняет принятые юридические документы.
pub(super) fn insert_user(
    state: &Mutex<InMemoryState>,
    nickname: String,
    email: String,
    email_normalized: String,
    password_hash: Option<String>,
    legal_acceptance: RegistrationLegalAcceptance,
    now: DateTime<Utc>,
) -> Result<UserAccount, InsertUserError> {
    let mut state = state
        .lock()
        .map_err(|_| InsertUserError::Storage(poisoned()))?;
    if state
        .users
        .iter()
        .any(|user| user.account.nickname == nickname)
    {
        return Err(InsertUserError::Conflict(UserConflict::Nickname));
    }
    if state
        .users
        .iter()
        .any(|user| user.email_normalized == email_normalized)
    {
        return Err(InsertUserError::Conflict(UserConflict::Email));
    }

    let account = UserAccount {
        id: Uuid::new_v4(),
        nickname,
        email,
        password_hash,
        avatar_image_id: None,
        registered_at: now,
        nickname_updated_at: now,
    };
    state.users.push(InMemoryUser {
        account: account.clone(),
        email_normalized,
    });
    state.legal_acceptances.extend([
        (
            account.id,
            "terms".to_owned(),
            legal_acceptance.terms_version,
            legal_acceptance.acceptance_source.clone(),
            now,
        ),
        (
            account.id,
            "privacy_policy".to_owned(),
            legal_acceptance.privacy_policy_version,
            legal_acceptance.acceptance_source.clone(),
            now,
        ),
        (
            account.id,
            "personal_data_consent".to_owned(),
            legal_acceptance.personal_data_consent_version,
            legal_acceptance.acceptance_source,
            now,
        ),
    ]);

    Ok(account)
}

/// Находит пользователя по нормализованному email.
pub(super) fn find_user_by_email(
    state: &Mutex<InMemoryState>,
    email_normalized: &str,
) -> anyhow::Result<Option<UserAccount>> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state
        .users
        .iter()
        .find(|user| user.email_normalized == email_normalized)
        .map(|user| user.account.clone()))
}

/// Находит пользователя по идентификатору.
pub(super) fn find_user_by_id(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
) -> anyhow::Result<Option<UserAccount>> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state
        .users
        .iter()
        .find(|user| user.account.id == *user_id)
        .map(|user| user.account.clone()))
}

/// Ищет пользователей по никнейму.
pub(super) fn search_users_by_nickname(
    state: &Mutex<InMemoryState>,
    query: &str,
    limit: u64,
) -> anyhow::Result<Vec<UserAccount>> {
    let needle = query.to_lowercase();
    let limit = usize::try_from(limit).unwrap_or(20);
    let state = state.lock().map_err(|_| poisoned())?;
    let mut users = state
        .users
        .iter()
        .filter(|user| user.account.nickname.to_lowercase().contains(&needle))
        .map(|user| user.account.clone())
        .collect::<Vec<_>>();
    users.sort_by(|left, right| left.nickname.cmp(&right.nickname));
    users.truncate(limit);
    Ok(users)
}

/// Читает изображения аватаров выбранных пользователей.
pub(super) fn avatar_image_ids_by_user_ids(
    state: &Mutex<InMemoryState>,
    user_ids: &[Uuid],
) -> anyhow::Result<HashMap<Uuid, Uuid>> {
    let state = state.lock().map_err(|_| poisoned())?;
    Ok(state
        .users
        .iter()
        .filter(|user| user_ids.contains(&user.account.id))
        .filter_map(|user| {
            user.account
                .avatar_image_id
                .map(|image_id| (user.account.id, image_id))
        })
        .collect())
}
