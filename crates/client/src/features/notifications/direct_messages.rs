//! Общая обработка realtime-уведомлений о личных сообщениях.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::RealtimeHandle;
use crate::features::social::realtime::subscribe_social_ready_events;

/// Возвращает, требует ли личное сообщение внимания пользователя.
pub(super) fn requires_attention(application_is_focused: bool, conversation_is_open: bool) -> bool {
    !application_is_focused || !conversation_is_open
}

/// Поддерживает social-подписку, пока работает глобальный провайдер уведомлений.
pub(super) async fn keep_social_subscription_active(realtime: RealtimeHandle) {
    let mut ready_events = subscribe_social_ready_events(realtime);
    while ready_events.next().await.is_some() {
        debug!("глобальная realtime-подписка social активна для уведомлений");
    }
    warn!("задача глобальной realtime-подписки social завершилась");
}

#[cfg(test)]
mod tests;
