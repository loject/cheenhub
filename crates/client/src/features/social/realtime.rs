//! Realtime-подписка на social-события.

use cheenhub_contracts::realtime::{
    RealtimeEnvelope, RealtimeKind, RealtimeModule, SocialChanged, SocialKind, SocialReady,
    SubscribeSocial,
};
use dioxus::prelude::{debug, info, warn};
use futures_channel::mpsc;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeError, RealtimeHandle};
use crate::features::runtime::sleep_ms;

const SOCIAL_SUBSCRIBE_RETRY_MS: u32 = 1_000;

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

/// Поддерживает social-подписку после первого подключения и переподключений.
pub(crate) fn subscribe_social_ready_events(
    realtime: RealtimeHandle,
) -> mpsc::UnboundedReceiver<()> {
    let (sender, receiver) = mpsc::unbounded();
    dioxus::prelude::spawn(async move {
        let mut statuses = realtime.subscribe_connection_status();
        while let Some(status) = statuses.next().await {
            match status {
                RealtimeConnectionStatus::Connected(transport) => {
                    let mut attempt = 1_u32;
                    loop {
                        match subscribe_social(&realtime).await {
                            Ok(_) => {
                                info!(?transport, "social realtime subscription active");
                                if sender.unbounded_send(()).is_err() {
                                    return;
                                }
                                break;
                            }
                            Err(error) => {
                                if !matches!(
                                    realtime.connection_status(),
                                    RealtimeConnectionStatus::Connected(_)
                                ) {
                                    debug!(
                                        %error,
                                        "social realtime subscription postponed until reconnect"
                                    );
                                    break;
                                }
                                warn!(
                                    %error,
                                    attempt,
                                    retry_ms = SOCIAL_SUBSCRIBE_RETRY_MS,
                                    "failed to subscribe social realtime; retrying"
                                );
                                attempt = attempt.saturating_add(1);
                                sleep_ms(SOCIAL_SUBSCRIBE_RETRY_MS).await;
                            }
                        }
                    }
                }
                RealtimeConnectionStatus::Disconnected => {
                    debug!("waiting for realtime connection before social subscription");
                }
            }
        }

        warn!("realtime status subscription closed before social subscription task stopped");
    });

    receiver
}

/// Подписывается на входящие social-события текущей вкладки.
pub(crate) fn subscribe_social_events(
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
