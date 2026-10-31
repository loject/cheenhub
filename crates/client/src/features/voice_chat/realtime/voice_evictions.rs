//! Подписка на адресные уведомления о вытеснении голосового подключения.

use cheenhub_contracts::realtime::VoicePresenceEvicted;
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::RealtimeHandle;

use super::realtime_decode;

pub(crate) fn subscribe_voice_evictions(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<VoicePresenceEvicted> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(event) = realtime_decode::voice_presence_evicted(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    });
    receiver
}
