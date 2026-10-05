//! Управление владельцами хоста.
//!
//! Права владельца выдаются нескольким пользователям одновременно, поэтому
//! модуль хранит список, а не единственного владельца. Отзыв прав последнего
//! владельца запрещён: иначе настройки хоста станут недоступны навсегда.

use std::collections::HashMap;

use cheenhub_contracts::rest::{GrantHostOwnerRequest, HostOwnerSummary, HostOwnersResponse};
use chrono::{SecondsFormat, Utc};
use uuid::Uuid;

use crate::features::auth::application::auth_user;
use crate::features::auth::domain::UserAccount;
use crate::state::AppState;

use super::application::{self, HostSettingsError};
use super::domain::{HostOwner, RevokeHostOwnerOutcome};

/// Возвращает список владельцев хоста вместе с их профилями.
///
/// Профили читаются одним запросом для всех владельцев и авторов выдачи,
/// поэтому список не растёт по числу обращений к хранилищу.
pub(crate) async fn list_owners(
    state: &AppState,
    access_token: &str,
) -> Result<HostOwnersResponse, HostSettingsError> {
    let current_user_id = application::require_host_owner(state, access_token).await?;
    let owners = state.host_settings_store.load_host_owners().await?;
    let user_ids = owners
        .iter()
        .map(|owner| owner.user_id)
        .chain(owners.iter().filter_map(|owner| owner.granted_by_user_id))
        .collect::<Vec<_>>();
    let profiles = state
        .auth_store
        .find_users_by_ids(&user_ids)
        .await?
        .into_iter()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();

    let summaries = owners
        .iter()
        .map(|owner| summary(state, owner, &profiles, current_user_id))
        .collect();
    tracing::debug!(
        %current_user_id,
        owner_count = owners.len(),
        "listed host owners"
    );
    Ok(HostOwnersResponse { owners: summaries })
}
/// Выдаёт права владельца хоста пользователю, указанному email или идентификатором.
///
/// Проверка tombstone и запись прав удерживают lifecycle-блокировку аккаунта,
/// поэтому удаление аккаунта и выдача прав не могут одновременно завершиться.
pub(crate) async fn grant_owner(
    state: &AppState,
    access_token: &str,
    request: GrantHostOwnerRequest,
) -> Result<HostOwnersResponse, HostSettingsError> {
    let current_user_id = application::require_host_owner(state, access_token).await?;
    let target = resolve_target(state, request.user).await?;

    if target.id == current_user_id {
        return Err(HostSettingsError::BadRequest(
            "У тебя уже есть права владельца хоста.".to_owned(),
        ));
    }
    if state.host_settings_store.is_host_owner(target.id).await? {
        return Err(HostSettingsError::BadRequest(format!(
            "У пользователя {} уже есть права владельца хоста.",
            target.nickname
        )));
    }
    // Та же блокировка удерживается удалением аккаунта: права выдаются
    // либо до проверки владельца при удалении, либо после отказа по tombstone.
    let _lifecycle = state.auth_store.lock_account_lifecycle(&target.id).await?;
    if state
        .auth_store
        .account_deletion(&target.id)
        .await?
        .is_some()
    {
        tracing::warn!(%current_user_id, target_user_id = %target.id, "rejected host owner grant to deleted account");
        return Err(HostSettingsError::BadRequest(
            "У этого аккаунта удалён доступ к CheenHub.".to_owned(),
        ));
    }

    state
        .host_settings_store
        .grant_host_owner(HostOwner {
            user_id: target.id,
            granted_at: Utc::now(),
            granted_by_user_id: Some(current_user_id),
        })
        .await?;
    tracing::info!(
        %current_user_id,
        granted_to = %target.id,
        granted_to_nickname = %target.nickname,
        "granted host owner rights"
    );

    list_owners(state, access_token).await
}
/// Отзывает права владельца хоста, оставляя на хосте хотя бы одного владельца.
pub(crate) async fn revoke_owner(
    state: &AppState,
    access_token: &str,
    target_user_id: Uuid,
) -> Result<HostOwnersResponse, HostSettingsError> {
    let current_user_id = application::require_host_owner(state, access_token).await?;
    match state
        .host_settings_store
        .revoke_host_owner(target_user_id)
        .await?
    {
        RevokeHostOwnerOutcome::Missing => {
            tracing::warn!(%current_user_id, %target_user_id, "rejected host owner revocation for a user without owner rights");
            return Err(HostSettingsError::BadRequest(
                "У этого пользователя нет прав владельца хоста.".to_owned(),
            ));
        }
        RevokeHostOwnerOutcome::LastOwner => {
            tracing::warn!(%current_user_id, %target_user_id, "rejected host owner revocation that would leave the host without an owner");
            return Err(HostSettingsError::BadRequest(
                "На хосте должен остаться хотя бы один владелец. Сначала выдай права другому пользователю.".to_owned(),
            ));
        }
        RevokeHostOwnerOutcome::Revoked => {}
    }
    tracing::info!(
        %current_user_id,
        revoked_from = %target_user_id,
        "revoked host owner rights"
    );

    list_owners(state, access_token).await
}

/// Находит пользователя по идентификатору или по email.
///
/// Формат ввода определяет разбор: корректный UUID ищется как идентификатор,
/// всё остальное считается email в нижнем регистре, как при регистрации.
async fn resolve_target(state: &AppState, input: String) -> Result<UserAccount, HostSettingsError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(HostSettingsError::BadRequest(
            "Укажи email или идентификатор пользователя.".to_owned(),
        ));
    }

    let user = match Uuid::parse_str(input) {
        Ok(user_id) => state.auth_store.find_user_by_id(&user_id).await?,
        Err(_) => {
            state
                .auth_store
                .find_user_by_email(&input.to_lowercase())
                .await?
        }
    };

    user.ok_or_else(|| {
        HostSettingsError::BadRequest(
            "Пользователь с такими данными не найден в CheenHub.".to_owned(),
        )
    })
}

/// Приводит запись о правах и профиль владельца к контракту REST.
fn summary(
    state: &AppState,
    owner: &HostOwner,
    profiles: &HashMap<Uuid, UserAccount>,
    current_user_id: Uuid,
) -> HostOwnerSummary {
    // Профиль удалённого аккаунта каскадно удаляется вместе с правами, но подстраховка
    // сохраняет интерфейс рабочим, если запись всё же осталась без профиля.
    let fallback = UserAccount {
        id: owner.user_id,
        nickname: "Удалённый пользователь".to_owned(),
        email: String::new(),
        password_hash: None,
        avatar_image_id: None,
        registered_at: owner.granted_at,
        nickname_updated_at: owner.granted_at,
    };
    let profile = auth_user(state, profiles.get(&owner.user_id).unwrap_or(&fallback));
    let granted_by_nickname = owner
        .granted_by_user_id
        .and_then(|granted_by| profiles.get(&granted_by).map(|user| user.nickname.clone()));

    HostOwnerSummary {
        user_id: profile.id,
        nickname: profile.nickname,
        email: profile.email,
        avatar_url: profile.avatar_url,
        granted_at: owner.granted_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        granted_by_nickname,
        is_current_user: owner.user_id == current_user_id,
    }
}

#[cfg(test)]
mod tests;
