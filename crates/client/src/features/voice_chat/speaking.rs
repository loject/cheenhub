//! Управление индикаторами говорящих участников голосовой комнаты.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use uuid::Uuid;

use crate::features::runtime::sleep_ms;

const SPEAKING_RELEASE_TIMEOUT_MS: u32 = 450;

/// Активность одного говорящего участника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpeakingUserActivity {
    /// Идентификатор аккаунта.
    user_id: String,
    /// Сессия устройства, если её можно определить по медиапакету.
    session_id: Option<Uuid>,
}

/// Возвращает идентификаторы участников, сейчас помеченных говорящими.
pub(super) fn user_ids(speaking_users: Signal<Vec<SpeakingUserActivity>>) -> Vec<String> {
    speaking_users()
        .into_iter()
        .map(|activity| activity.user_id)
        .collect()
}

/// Возвращает независимые от аккаунта метки активности каждой голосовой сессии.
pub(super) fn sessions(
    speaking_users: Signal<Vec<SpeakingUserActivity>>,
) -> Vec<(String, Option<Uuid>)> {
    speaking_users()
        .into_iter()
        .map(|activity| (activity.user_id, activity.session_id))
        .collect()
}

/// Помечает одного участника говорящим до истечения окна без новых frame.
pub(super) fn mark_user_speaking(
    speaking_users: Signal<Vec<SpeakingUserActivity>>,
    speaking_generations: Rc<RefCell<HashMap<String, u64>>>,
    user_id: String,
    session_id: Option<Uuid>,
) {
    let key = format!("{user_id}/{}", session_id.unwrap_or_default());
    let generation = {
        let mut generations = speaking_generations.borrow_mut();
        let generation = generations.entry(key.clone()).or_insert(0);
        *generation = generation.saturating_add(1);
        *generation
    };

    let mut next_users = speaking_users();
    if next_users
        .iter()
        .any(|activity| activity.user_id == user_id && activity.session_id == session_id)
    {
        return;
    }
    next_users.push(SpeakingUserActivity {
        user_id: user_id.clone(),
        session_id,
    });
    let mut speaking_users = speaking_users;
    speaking_users.set(next_users);

    spawn(async move {
        let mut observed_generation = generation;
        loop {
            sleep_ms(SPEAKING_RELEASE_TIMEOUT_MS).await;
            let latest_generation = speaking_generations.borrow().get(&key).copied();
            match latest_generation {
                Some(latest_generation) if latest_generation != observed_generation => {
                    observed_generation = latest_generation;
                }
                Some(_) => {
                    speaking_generations.borrow_mut().remove(&key);
                    let mut next_users = speaking_users();
                    let previous_len = next_users.len();
                    next_users.retain(|activity| {
                        activity.user_id != user_id || activity.session_id != session_id
                    });
                    if next_users.len() != previous_len {
                        speaking_users.set(next_users);
                    }
                    break;
                }
                None => break,
            }
        }
    });
}

/// Очищает все удаленные индикаторы речи.
pub(super) fn clear_speaking_users(
    speaking_users: Signal<Vec<SpeakingUserActivity>>,
    speaking_generations: Rc<RefCell<HashMap<String, u64>>>,
) {
    speaking_generations.borrow_mut().clear();
    let mut speaking_users = speaking_users;
    speaking_users.set(Vec::new());
}
