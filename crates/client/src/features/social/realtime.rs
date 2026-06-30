//! Realtime-подписка на social-события.

use cheenhub_contracts::realtime::{
    RealtimeEnvelope, RealtimeKind, RealtimeModule, SocialChanged, SocialKind, SocialReady,
    SubscribeSocial,
};
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Открывает reliable stream social-модуля для текущей вкладки.
pub(super) async fn subscribe_social(
    realtime: &RealtimeHandle,
) -> Result<SocialReady, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::Subscribe),
            SubscribeSocial,
        )
        .await
}

/// Подписывается на входящие social-события текущей вкладки.
pub(super) fn subscribe_social_events(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<SocialChanged> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(event) = decode_social_event(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    });

    receiver
}

fn decode_social_event(envelope: RealtimeEnvelope) -> Option<SocialChanged> {
    if envelope.module != RealtimeModule::Social {
        return None;
    }
    match envelope.kind {
        RealtimeKind::Social(SocialKind::Changed) => {
            serde_json::from_value::<SocialChanged>(envelope.payload).ok()
        }
        _ => None,
    }
}
