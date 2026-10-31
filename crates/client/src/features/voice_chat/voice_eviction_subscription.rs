//! Подписка на адресные уведомления сервера о вытеснении голосового подключения.

use dioxus::prelude::*;
use futures_util::StreamExt;

use super::realtime;
use super::state::VoiceConnectionHandle;
use crate::features::realtime::RealtimeHandle;

pub(super) fn use_voice_eviction_subscription(
    realtime: RealtimeHandle,
    handle: VoiceConnectionHandle,
) {
    use_hook(move || {
        spawn(async move {
            let mut events = realtime::subscribe_voice_evictions(&realtime);
            while let Some(event) = events.next().await {
                handle.apply_eviction(event);
            }
        })
    });
}
