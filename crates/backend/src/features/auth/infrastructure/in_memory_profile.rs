//! In-memory user profile update helpers.

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::features::auth::domain::UserAccount;
use crate::features::auth::infrastructure::in_memory::model::InMemoryState;
use crate::features::auth::infrastructure::{
    UpdateUserNicknameError, UserConflict, in_memory::poisoned,
};

/// Updates a user's public nickname.
pub(super) fn update_user_nickname(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    session_id: &Uuid,
    nickname: String,
    now: DateTime<Utc>,
    cooldown: Duration,
) -> Result<Option<UserAccount>, UpdateUserNicknameError> {
    let mut state = state
        .lock()
        .map_err(|_| UpdateUserNicknameError::Storage(poisoned()))?;
    if state
        .users
        .iter()
        .any(|user| user.account.id != *user_id && user.account.nickname == nickname)
    {
        return Err(UpdateUserNicknameError::Conflict(UserConflict::Nickname));
    }
    let (old_nickname, account) = {
        let Some(user) = state
            .users
            .iter_mut()
            .find(|user| user.account.id == *user_id)
        else {
            return Ok(None);
        };

        let next_allowed_at = user.account.nickname_updated_at + cooldown;
        if user.account.nickname != nickname && now < next_allowed_at {
            return Err(UpdateUserNicknameError::Cooldown { next_allowed_at });
        }

        let old_nickname = user.account.nickname.clone();
        user.account.nickname = nickname.clone();
        user.account.nickname_updated_at = now;
        (old_nickname, user.account.clone())
    };
    if old_nickname != nickname {
        state.user_nickname_history.push((
            Uuid::new_v4(),
            *user_id,
            *session_id,
            old_nickname,
            nickname,
            now,
        ));
    }

    Ok(Some(account))
}

/// Updates a user's current avatar image identifier.
pub(super) fn update_user_avatar_image_id(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    image_id: Uuid,
) -> anyhow::Result<Option<UserAccount>> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let Some(user) = state
        .users
        .iter_mut()
        .find(|user| user.account.id == *user_id)
    else {
        return Ok(None);
    };
    user.account.avatar_image_id = Some(image_id);
    Ok(Some(user.account.clone()))
}

/// Updates a user's password hash and records the password change trace.
pub(super) fn change_user_password(
    state: &Mutex<InMemoryState>,
    user_id: &Uuid,
    session_id: &Uuid,
    password_hash: String,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    if let Some(user) = state
        .users
        .iter_mut()
        .find(|user| user.account.id == *user_id)
    {
        user.account.password_hash = Some(password_hash);
    }
    state
        .user_password_change_trace
        .push((Uuid::new_v4(), *user_id, *session_id, now));

    Ok(())
}
