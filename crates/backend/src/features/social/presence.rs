//! Присутствие друзей: преобразование активности сессий и адресная рассылка.
//!
//! RealtimeHub сообщает только о соединениях; social проверяет отношения дружбы.

use std::collections::HashSet;

use cheenhub_contracts::presence::UserPresence;
use cheenhub_contracts::realtime::{
    PresenceChanged, RealtimeKind, RealtimeModule, SocialChangeReason, SocialKind,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::domain::FriendshipStatus;
use super::realtime::notify_social_changed;
use crate::realtime::hub::{SessionActivity, SessionActivityChange};
use crate::state::AppState;

/// Преобразует данные транспортных сессий в публичный контракт присутствия.
pub(super) fn public_presence(activity: SessionActivity) -> UserPresence {
    if activity.online {
        UserPresence::Online
    } else {
        UserPresence::Offline {
            last_seen_at: activity.last_seen_at.map(|time| time.to_rfc3339()),
        }
    }
}

/// Подключает обработчик событий сессий до начала приёма клиентов.
pub(crate) async fn spawn_presence_updates(state: AppState) {
    let receiver = state.realtime_hub.subscribe_session_activity().await;
    tokio::spawn(async move { run_presence_updates(state, receiver).await });
    tracing::info!("social presence updates enabled");
}

async fn run_presence_updates(
    state: AppState,
    mut receiver: broadcast::Receiver<SessionActivityChange>,
) {
    loop {
        match receiver.recv().await {
            Ok(change) => notify_presence_change(&state, change).await,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "social presence events lagged; requesting refresh");
                let recipients = state
                    .realtime_hub
                    .module_recipients(RealtimeModule::Social)
                    .into_iter()
                    .map(|recipient| recipient.user_id)
                    .collect::<Vec<_>>();
                notify_social_changed(&state, &recipients, SocialChangeReason::Friends, None).await;
            }
            Err(broadcast::error::RecvError::Closed) => {
                tracing::warn!("social presence event source closed");
                break;
            }
        }
    }
}

/// Рассылает изменение присутствия только пользователям с подтверждённой дружбой.
pub(crate) async fn notify_presence_change(state: &AppState, change: SessionActivityChange) {
    let candidates = state
        .realtime_hub
        .module_recipients(RealtimeModule::Social)
        .into_iter()
        .map(|recipient| recipient.user_id)
        .filter(|user_id| *user_id != change.user_id)
        .collect::<HashSet<_>>();
    if candidates.is_empty() {
        return;
    }
    let candidate_ids = candidates.into_iter().collect::<Vec<_>>();
    let friendships = match state
        .social_store
        .friendships_with_user(&change.user_id, &candidate_ids)
        .await
    {
        Ok(friendships) => friendships,
        Err(error) => {
            tracing::warn!(user_id = %change.user_id, %error, "failed to resolve presence recipients");
            return;
        }
    };
    let recipients = friendships
        .into_iter()
        .filter(|friendship| friendship.status == FriendshipStatus::Accepted)
        .map(|friendship| {
            if friendship.user_low_id == change.user_id {
                friendship.user_high_id
            } else {
                friendship.user_low_id
            }
        })
        .collect::<Vec<Uuid>>();
    if recipients.is_empty() {
        return;
    }
    tracing::debug!(user_id = %change.user_id, recipient_count = recipients.len(), online = change.activity.online, "fanning out friend presence update");
    state
        .realtime_hub
        .fanout_to_user_streams(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::PresenceChanged),
            &recipients,
            PresenceChanged {
                user_id: change.user_id.to_string(),
                presence: public_presence(change.activity),
            },
        )
        .await;
}
