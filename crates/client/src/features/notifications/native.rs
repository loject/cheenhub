//! Платформенная обертка для уведомлений.
//!
//! На веб-платформе делегирует в web-реализацию.
//! На desktop-платформе проигрывает звуки личных сообщений.

#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::{NotificationsProvider, application_is_focused};

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
mod impl_ {
    use cheenhub_contracts::realtime::SocialChangeReason;
    use dioxus::desktop::use_window;
    use dioxus::prelude::*;
    use futures_util::StreamExt;

    use crate::features::app::active_room::ActiveRoomContext;
    use crate::features::app::current_user::CurrentUserContext;
    use crate::features::audio_playback::{AudioPlaybackHandle, NotificationSound};
    use crate::features::realtime::RealtimeHandle;
    use crate::features::runtime::sleep_ms;
    use crate::features::social::realtime::{
        subscribe_social_events, subscribe_social_ready_events,
    };

    use super::super::direct_messages::{extract_notification, load_initial_unread_snapshot};
    use super::super::focus::ApplicationFocusContext;

    /// Подписывается на личные сообщения и проигрывает звук, когда они требуют внимания.
    #[component]
    pub(crate) fn NotificationsProvider(children: Element) -> Element {
        let realtime = use_context::<RealtimeHandle>();
        let current_user = use_context::<CurrentUserContext>();
        let active_room = use_context::<ActiveRoomContext>();
        let playback = use_context::<AudioPlaybackHandle>();
        let window = use_window();
        let unread_snapshot = use_signal(Vec::<(String, i64)>::new);
        let focus_state = use_signal(application_is_focused);
        use_context_provider(move || ApplicationFocusContext::new(focus_state));

        use_hook(move || {
            spawn(track_application_focus(focus_state));
            spawn(keep_social_subscription_active(realtime.clone()));
            spawn(load_initial_unread_snapshot(unread_snapshot));
            spawn(listen_for_dm_sounds(
                realtime,
                current_user,
                active_room,
                playback,
                window,
                unread_snapshot,
            ));
        });

        rsx! {
            {children}
        }
    }

    async fn track_application_focus(mut focus_state: Signal<bool>) {
        loop {
            let next_focused = application_is_focused();
            if focus_state() != next_focused {
                focus_state.set(next_focused);
                debug!(next_focused, "updated desktop application focus state");
            }
            sleep_ms(250).await;
        }
    }

    async fn keep_social_subscription_active(realtime: RealtimeHandle) {
        let mut ready_events = subscribe_social_ready_events(realtime);
        while ready_events.next().await.is_some() {
            debug!("global social realtime subscription is active for notifications");
        }
        warn!("global social realtime subscription task stopped");
    }

    /// Возвращает `true`, если окно desktop-клиента видно и находится в фокусе.
    pub(crate) fn application_is_focused() -> bool {
        let window = dioxus::desktop::window();
        window.is_visible() && window.is_focused()
    }

    async fn listen_for_dm_sounds(
        realtime: RealtimeHandle,
        current_user: CurrentUserContext,
        active_room: ActiveRoomContext,
        playback: AudioPlaybackHandle,
        window: dioxus::desktop::DesktopContext,
        unread_snapshot: Signal<Vec<(String, i64)>>,
    ) {
        let mut receiver = subscribe_social_events(&realtime);
        while let Some(event) = receiver.next().await {
            if event.reason != SocialChangeReason::DirectMessages {
                continue;
            }

            let Some(notification) =
                extract_notification(&event, &current_user, unread_snapshot).await
            else {
                continue;
            };
            let application_is_focused = window.is_visible() && window.is_focused();
            let conversation_is_open = active_room.conversation_id().as_deref()
                == Some(notification.conversation_id.as_str());
            if application_is_focused && conversation_is_open {
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

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub(crate) use impl_::NotificationsProvider;
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub(crate) use impl_::application_is_focused;

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
pub(crate) use super::unsupported::NotificationsProvider;
#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
pub(crate) use super::unsupported::application_is_focused;
