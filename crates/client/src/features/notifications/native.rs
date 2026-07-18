//! Платформенная обертка для уведомлений.
//!
//! На веб-платформе делегирует в web-реализацию.
//! На desktop-платформе проигрывает звуки личных сообщений.

#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::NotificationsProvider;

#[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
pub(crate) use super::android::NotificationsProvider;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "desktop"
))]
mod impl_ {
    use dioxus::prelude::*;
    use futures_util::StreamExt;

    use crate::features::app::active_room::ActiveRoomContext;
    use crate::features::application_focus::application_is_focused;
    use crate::features::audio_playback::{AudioPlaybackHandle, NotificationSound};
    use crate::features::realtime::RealtimeHandle;
    use crate::features::social::realtime::subscribe_direct_message_events;

    use super::super::direct_messages::{keep_social_subscription_active, requires_attention};

    /// Подписывается на личные сообщения и проигрывает звук, когда они требуют внимания.
    #[component]
    pub(crate) fn NotificationsProvider(children: Element) -> Element {
        let realtime = use_context::<RealtimeHandle>();
        let active_room = use_context::<ActiveRoomContext>();
        let playback = use_context::<AudioPlaybackHandle>();

        use_hook(move || {
            spawn(keep_social_subscription_active(realtime.clone()));
            spawn(listen_for_dm_sounds(realtime, active_room, playback));
        });

        rsx! {
            {children}
        }
    }

    async fn listen_for_dm_sounds(
        realtime: RealtimeHandle,
        active_room: ActiveRoomContext,
        playback: AudioPlaybackHandle,
    ) {
        let mut receiver = subscribe_direct_message_events(&realtime);
        while let Some(notification) = receiver.next().await {
            let application_is_focused = application_is_focused();
            let conversation_is_open = active_room.conversation_id().as_deref()
                == Some(notification.conversation_id.as_str());
            if !requires_attention(application_is_focused, conversation_is_open) {
                debug!(
                    conversation_id = %notification.conversation_id,
                    "suppressed direct message notification sound for focused open conversation"
                );
                continue;
            }

            debug!(
                conversation_id = %notification.conversation_id,
                application_is_focused,
                conversation_is_open,
                "playing direct message notification sound"
            );
            playback.play_notification_sound(NotificationSound::MessageReceived);
        }
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(feature = "desktop")
))]
pub(crate) use super::unsupported::NotificationsProvider;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    feature = "desktop"
))]
pub(crate) use impl_::NotificationsProvider;
