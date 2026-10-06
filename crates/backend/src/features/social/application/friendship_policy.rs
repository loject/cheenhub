//! Правила проверки дружбы между пользователями.
//!
//! Модуль собирает все решения, которые опираются на статус связи дружбы: вид
//! отношения для поиска пользователей и допуск к личным сообщениям и голосовым
//! звонкам. Правило проверки принятой дружбы одно, а различаются только тексты
//! отказа, зависящие от действия, поэтому формулировки передаются вызывающей
//! стороной.

use cheenhub_contracts::rest::UserRelationStatus;
use std::collections::HashMap;
use uuid::Uuid;

use crate::features::social::domain::{Friendship, FriendshipStatus};
use crate::features::social::error::SocialError;
use crate::state::AppState;

/// Возвращает отношение текущего пользователя к набору кандидатов одним запросом.
///
/// Отношение вычисляется по записям дружбы пачкой, а не по отдельной проверке на
/// каждого кандидата: выдача поиска ограничена небольшим лимитом, поэтому построчная
/// проверка дала бы до двадцати последовательных запросов на один поиск.
///
/// Кандидаты без записи дружбы в результат не попадают, что означает «отношений нет».
pub(crate) async fn relation_statuses(
    state: &AppState,
    current_user_id: &Uuid,
    other_user_ids: &[Uuid],
) -> Result<HashMap<Uuid, UserRelationStatus>, SocialError> {
    let friendships = state
        .social_store
        .friendships_with_user(current_user_id, other_user_ids)
        .await
        .map_err(SocialError::Internal)?;

    let mut statuses = HashMap::with_capacity(friendships.len());
    for friendship in friendships {
        let peer_user_id = other_friendship_user_id(&friendship, current_user_id);
        if let Some(status) = relation_status(current_user_id, friendship) {
            statuses.insert(peer_user_id, status);
        }
    }
    Ok(statuses)
}

/// Строит статус отношения по записи дружбы, где `None` означает «отношений нет».
///
/// Отклоненные и отмененные заявки не считаются отношением: пользователь снова может
/// отправить заявку, поэтому показывать ему постоянную метку было бы вводящим в
/// заблуждение.
fn relation_status(current_user_id: &Uuid, friendship: Friendship) -> Option<UserRelationStatus> {
    Some(match friendship.status {
        FriendshipStatus::Accepted => UserRelationStatus::Friends,
        FriendshipStatus::Pending if friendship.requester_user_id == *current_user_id => {
            UserRelationStatus::PendingOutgoing
        }
        FriendshipStatus::Pending => UserRelationStatus::PendingIncoming,
        FriendshipStatus::Declined | FriendshipStatus::Cancelled => return None,
    })
}

/// Возвращает собеседника по записи дружбы относительно текущего пользователя.
///
/// Пара хранится в упорядоченном виде, поэтому собеседник находится сравнением
/// с обеими половинами пары.
fn other_friendship_user_id(friendship: &Friendship, current_user_id: &Uuid) -> Uuid {
    if friendship.user_low_id == *current_user_id {
        friendship.user_high_id
    } else {
        friendship.user_low_id
    }
}

/// Проверяет, что пользователи друзья, и возвращает заданный текст ошибки иначе.
///
/// Единая точка проверки принятой дружбы для голосовых звонков и личных сообщений.
/// Отказ логируется на уровне warn, потому что обычно означает попытку обратиться к
/// диалогу, к которому у пользователя нет доступа.
pub(crate) async fn ensure_accepted_friendship(
    state: &AppState,
    current_user_id: &Uuid,
    friend_user_id: &Uuid,
    rejection: &str,
) -> Result<(), SocialError> {
    if accepted_friendship_exists(state, current_user_id, friend_user_id).await? {
        Ok(())
    } else {
        tracing::warn!(
            user_id = %current_user_id,
            friend_user_id = %friend_user_id,
            %rejection,
            "rejected direct message action for non-friends"
        );
        Err(SocialError::Unauthorized(rejection.to_owned()))
    }
}

/// Сообщает, установлена ли между пользователями принятая дружба.
///
/// Отдельная функция для сценариев, где отказ не нужен, а достаточно признака,
/// например при построении списка доступных голосовых диалогов.
pub(crate) async fn accepted_friendship_exists(
    state: &AppState,
    left_user_id: &Uuid,
    right_user_id: &Uuid,
) -> Result<bool, SocialError> {
    let friendship = state
        .social_store
        .friendship_between(left_user_id, right_user_id)
        .await
        .map_err(SocialError::Internal)?;
    Ok(friendship.is_some_and(|friendship| friendship.status == FriendshipStatus::Accepted))
}
