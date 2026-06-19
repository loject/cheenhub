//! Управление индикаторами говорящих участников голосовой комнаты.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const SPEAKING_RELEASE_TIMEOUT_MS: u32 = 450;

/// Активность одного говорящего участника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpeakingUserActivity {
    /// Идентификатор участника.
    user_id: String,
}

/// Возвращает идентификаторы участников, сейчас помеченных говорящими.
pub(super) fn user_ids(speaking_users: Signal<Vec<SpeakingUserActivity>>) -> Vec<String> {
    speaking_users()
        .into_iter()
        .map(|activity| activity.user_id)
        .collect()
}

/// Помечает одного участника говорящим до истечения окна без новых frame.
pub(super) fn mark_user_speaking(
    speaking_users: Signal<Vec<SpeakingUserActivity>>,
    speaking_generations: Rc<RefCell<HashMap<String, u64>>>,
    user_id: String,
) {
    let generation = {
        let mut generations = speaking_generations.borrow_mut();
        let generation = generations.entry(user_id.clone()).or_insert(0);
        *generation = generation.saturating_add(1);
        *generation
    };

    let mut next_users = speaking_users();
    if next_users
        .iter()
        .any(|activity| activity.user_id == user_id)
    {
        return;
    }
    next_users.push(SpeakingUserActivity {
        user_id: user_id.clone(),
    });
    let mut speaking_users = speaking_users;
    speaking_users.set(next_users);

    spawn(async move {
        let mut observed_generation = generation;
        loop {
            TimeoutFuture::new(SPEAKING_RELEASE_TIMEOUT_MS).await;
            let latest_generation = speaking_generations.borrow().get(&user_id).copied();
            match latest_generation {
                Some(latest_generation) if latest_generation != observed_generation => {
                    observed_generation = latest_generation;
                }
                Some(_) => {
                    speaking_generations.borrow_mut().remove(&user_id);
                    let mut next_users = speaking_users();
                    let previous_len = next_users.len();
                    next_users.retain(|activity| activity.user_id != user_id);
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
