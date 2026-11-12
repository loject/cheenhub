//! Серверные правила входящих контактов и управление приватностью.

use std::collections::HashMap;

use cheenhub_contracts::realtime::SocialChangeReason;
use cheenhub_contracts::rest::{
    BlockedUserSummary, DirectMessageAudience, DmBlockStatus, IncomingContactAudience,
    ListBlockedUsersResponse, UserPrivacySettings,
};
use chrono::Utc;
use uuid::Uuid;

use super::friendship_policy::accepted_friendship_exists;
use crate::features::auth::application::auth_user;
use crate::features::social::domain::{IncomingContactRule, PrivacySettings};
use crate::features::social::error::SocialError;
use crate::features::social::realtime::notify_social_changed;
use crate::features::social::support::{ensure_user_active, ensure_user_exists};
use crate::state::AppState;

/// Проверяет блокировку пары в обе стороны, возвращая понятную ошибку отправителю.
pub(crate) async fn ensure_not_blocked(
    state: &AppState,
    actor: &Uuid,
    recipient: &Uuid,
) -> Result<(), SocialError> {
    let blocks = state
        .social_store
        .user_blocks_between(actor, recipient)
        .await
        .map_err(SocialError::Internal)?;
    if blocks
        .iter()
        .any(|row| row.unblocked_at.is_none() && row.blocker_user_id == *recipient)
    {
        tracing::debug!(%actor, %recipient, "rejected contact by recipient block");
        return Err(SocialError::Unauthorized(
            "Пользователь вас заблокировал.".to_owned(),
        ));
    }
    if blocks
        .iter()
        .any(|row| row.unblocked_at.is_none() && row.blocker_user_id == *actor)
    {
        return Err(SocialError::Conflict(
            "Вы заблокировали этого пользователя.".to_owned(),
        ));
    }
    Ok(())
}

/// Сообщает, действует ли хотя бы одна блокировка между пользователями.
pub(crate) async fn users_blocked(
    state: &AppState,
    left: &Uuid,
    right: &Uuid,
) -> Result<bool, SocialError> {
    Ok(state
        .social_store
        .user_blocks_between(left, right)
        .await
        .map_err(SocialError::Internal)?
        .iter()
        .any(|row| row.unblocked_at.is_none()))
}

/// Получает состояния блокировки собеседников одним обращением к хранилищу.
pub(crate) async fn block_statuses_for_peers(
    state: &AppState,
    user_id: &Uuid,
    peer_ids: &[Uuid],
) -> Result<HashMap<Uuid, DmBlockStatus>, SocialError> {
    if peer_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let blocks = state
        .social_store
        .active_blocks_with_peers(user_id, peer_ids)
        .await
        .map_err(SocialError::Internal)?;
    let mut directions = HashMap::<Uuid, (bool, bool)>::new();
    for block in blocks {
        let blocked_by_me = block.blocker_user_id == *user_id;
        let peer_id = if blocked_by_me {
            block.blocked_user_id
        } else {
            block.blocker_user_id
        };
        let direction = directions.entry(peer_id).or_default();
        if blocked_by_me {
            direction.0 = true;
        } else {
            direction.1 = true;
        }
    }
    Ok(directions
        .into_iter()
        .map(|(peer_id, (mine, theirs))| {
            let status = match (mine, theirs) {
                (true, true) => DmBlockStatus::Mutual,
                (true, false) => DmBlockStatus::BlockedByMe,
                (false, true) => DmBlockStatus::BlockedByPeer,
                (false, false) => DmBlockStatus::None,
            };
            (peer_id, status)
        })
        .collect())
}

/// Допускает ЛС с учётом дружбы, последнего ограничения и разблокировки.
pub(crate) async fn ensure_incoming_dm_allowed(
    state: &AppState,
    sender: &Uuid,
    recipient: &Uuid,
    conversation_id: Option<&Uuid>,
) -> Result<(), SocialError> {
    ensure_not_blocked(state, sender, recipient).await?;
    let settings = state
        .social_store
        .privacy_settings(recipient)
        .await
        .map_err(SocialError::Internal)?;
    if !settings.direct_messages_friends_only
        || accepted_friendship_exists(state, sender, recipient).await?
    {
        return Ok(());
    }
    let Some(conversation_id) = conversation_id else {
        return Err(dm_rejected());
    };
    let Some(mut cutoff) = settings.direct_messages_restricted_since else {
        return Err(dm_rejected());
    };
    // Разблокировка отменяет старые разрешения продолжить диалог в обоих направлениях.
    for row in state
        .social_store
        .user_blocks_between(sender, recipient)
        .await
        .map_err(SocialError::Internal)?
    {
        if let Some(unblocked_at) = row.unblocked_at {
            cutoff = cutoff.max(unblocked_at);
        }
    }
    if state
        .social_store
        .has_dm_message_after(conversation_id, recipient, cutoff)
        .await
        .map_err(SocialError::Internal)?
    {
        Ok(())
    } else {
        Err(dm_rejected())
    }
}

fn dm_rejected() -> SocialError {
    SocialError::Unauthorized(
        "Пользователь принимает личные сообщения только от друзей.".to_owned(),
    )
}

/// Проверяет правило входящих личных звонков у вызываемого участника.
pub(crate) async fn ensure_incoming_call_allowed(
    state: &AppState,
    caller: &Uuid,
    recipient: &Uuid,
) -> Result<(), SocialError> {
    ensure_not_blocked(state, caller, recipient).await?;
    let settings = state
        .social_store
        .privacy_settings(recipient)
        .await
        .map_err(SocialError::Internal)?;
    ensure_contact_rule(
        state,
        caller,
        recipient,
        settings.direct_calls,
        "Пользователь не принимает личные звонки.",
    )
    .await
}

/// Проверяет право пригласить получателя в серверную голосовую комнату.
pub(crate) async fn ensure_incoming_room_invite_allowed(
    state: &AppState,
    caller: &Uuid,
    recipient: &Uuid,
) -> Result<(), SocialError> {
    ensure_not_blocked(state, caller, recipient).await?;
    let settings = state
        .social_store
        .privacy_settings(recipient)
        .await
        .map_err(SocialError::Internal)?;
    ensure_contact_rule(
        state,
        caller,
        recipient,
        settings.voice_room_invites,
        "Пользователь не принимает приглашения в голосовые комнаты.",
    )
    .await
}

async fn ensure_contact_rule(
    state: &AppState,
    caller: &Uuid,
    recipient: &Uuid,
    rule: IncomingContactRule,
    rejection: &str,
) -> Result<(), SocialError> {
    match rule {
        IncomingContactRule::Everyone => Ok(()),
        IncomingContactRule::Friends
            if accepted_friendship_exists(state, caller, recipient).await? =>
        {
            Ok(())
        }
        IncomingContactRule::Friends | IncomingContactRule::Nobody => {
            Err(SocialError::Unauthorized(rejection.to_owned()))
        }
    }
}

/// Загружает настройки пользователя для REST API.
pub(crate) async fn get_privacy_settings(
    state: &AppState,
    user_id: &Uuid,
) -> Result<UserPrivacySettings, SocialError> {
    let settings = state
        .social_store
        .privacy_settings(user_id)
        .await
        .map_err(SocialError::Internal)?;
    Ok(to_contract(settings))
}

/// Сохраняет три независимых правила и сбрасывает старые разрешения ЛС при ужесточении.
pub(crate) async fn update_privacy_settings(
    state: &AppState,
    user_id: &Uuid,
    request: UserPrivacySettings,
) -> Result<UserPrivacySettings, SocialError> {
    let previous = state
        .social_store
        .privacy_settings(user_id)
        .await
        .map_err(SocialError::Internal)?;
    let now = Utc::now();
    let friends_only = request.direct_messages == DirectMessageAudience::Friends;
    let restricted_since = if friends_only {
        if previous.direct_messages_friends_only {
            previous.direct_messages_restricted_since.or(Some(now))
        } else {
            Some(now)
        }
    } else {
        None
    };
    let settings = PrivacySettings {
        direct_messages_friends_only: friends_only,
        direct_calls: to_domain_rule(request.direct_calls),
        voice_room_invites: to_domain_rule(request.voice_room_invites),
        direct_messages_restricted_since: restricted_since,
    };
    state
        .social_store
        .save_privacy_settings(user_id, settings, now)
        .await
        .map_err(SocialError::Internal)?;
    tracing::info!(%user_id, direct_messages_friends_only = friends_only,
        direct_calls = ?settings.direct_calls, voice_room_invites = ?settings.voice_room_invites,
        "updated user privacy settings");
    Ok(to_contract(settings))
}

/// Возвращает список пользователей, заблокированных владельцем.
pub(crate) async fn list_blocked_users(
    state: &AppState,
    user_id: &Uuid,
) -> Result<ListBlockedUsersResponse, SocialError> {
    let blocks = state
        .social_store
        .active_user_blocks(user_id)
        .await
        .map_err(SocialError::Internal)?;
    let ids = blocks
        .iter()
        .map(|block| block.blocked_user_id)
        .collect::<Vec<_>>();
    let accounts = state
        .auth_store
        .find_users_by_ids(&ids)
        .await
        .map_err(SocialError::Internal)?;
    let users = blocks
        .into_iter()
        .filter_map(|block| {
            let account = accounts
                .iter()
                .find(|account| account.id == block.blocked_user_id)?;
            let user = auth_user(state, account);
            Some(BlockedUserSummary {
                user_id: user.id,
                nickname: user.nickname,
                avatar_url: user.avatar_url,
                blocked_at: block.blocked_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(ListBlockedUsersResponse { users })
}

/// Устанавливает блокировку, отменяет связь дружбы и уведомляет оба списка.
pub(crate) async fn block_user(
    state: &AppState,
    blocker: &Uuid,
    blocked: &Uuid,
) -> Result<(), SocialError> {
    if blocker == blocked {
        return Err(SocialError::BadRequest(
            "Нельзя заблокировать себя.".to_owned(),
        ));
    }
    ensure_user_exists(state, blocked).await?;
    ensure_user_active(state, blocked).await?;
    let _contact_guard = state.contact_pair_guards.lock(blocker, blocked).await;
    let conversation_id = state
        .social_store
        .conversation_between(blocker, blocked)
        .await
        .map_err(|error| {
            tracing::error!(%blocker, %blocked, %error, "cannot resolve conversation before privacy block");
            SocialError::Internal(error)
        })?
        .map(|conversation| conversation.id);
    let now = Utc::now();
    state
        .social_store
        .block_user(blocker, blocked, now)
        .await
        .map_err(SocialError::Internal)?;
    tracing::info!(%blocker, %blocked, "blocked user and removed friendship");
    crate::features::voice_chat::application::terminate_contact_for_block(
        state,
        blocker,
        blocked,
        conversation_id,
    )
    .await;
    notify_social_changed(
        state,
        &[*blocker, *blocked],
        SocialChangeReason::Friends,
        None,
    )
    .await;
    notify_social_changed(
        state,
        &[*blocker, *blocked],
        SocialChangeReason::DirectMessages,
        None,
    )
    .await;
    Ok(())
}

/// Снимает направленную блокировку без восстановления дружбы.
pub(crate) async fn unblock_user(
    state: &AppState,
    blocker: &Uuid,
    blocked: &Uuid,
) -> Result<(), SocialError> {
    let _contact_guard = state.contact_pair_guards.lock(blocker, blocked).await;
    if !state
        .social_store
        .unblock_user(blocker, blocked, Utc::now())
        .await
        .map_err(SocialError::Internal)?
    {
        return Err(SocialError::NotFound("Блокировка не найдена.".to_owned()));
    }
    tracing::info!(%blocker, %blocked, "unblocked user");
    notify_social_changed(
        state,
        &[*blocker, *blocked],
        SocialChangeReason::Friends,
        None,
    )
    .await;
    notify_social_changed(
        state,
        &[*blocker, *blocked],
        SocialChangeReason::DirectMessages,
        None,
    )
    .await;
    Ok(())
}

fn to_contract(settings: PrivacySettings) -> UserPrivacySettings {
    UserPrivacySettings {
        direct_messages: if settings.direct_messages_friends_only {
            DirectMessageAudience::Friends
        } else {
            DirectMessageAudience::Everyone
        },
        direct_calls: to_contract_rule(settings.direct_calls),
        voice_room_invites: to_contract_rule(settings.voice_room_invites),
    }
}

fn to_contract_rule(rule: IncomingContactRule) -> IncomingContactAudience {
    match rule {
        IncomingContactRule::Everyone => IncomingContactAudience::Everyone,
        IncomingContactRule::Friends => IncomingContactAudience::Friends,
        IncomingContactRule::Nobody => IncomingContactAudience::Nobody,
    }
}

fn to_domain_rule(rule: IncomingContactAudience) -> IncomingContactRule {
    match rule {
        IncomingContactAudience::Everyone => IncomingContactRule::Everyone,
        IncomingContactAudience::Friends => IncomingContactRule::Friends,
        IncomingContactAudience::Nobody => IncomingContactRule::Nobody,
    }
}
