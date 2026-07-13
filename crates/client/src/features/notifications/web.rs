//! Web-реализация уведомлений о сообщениях.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use cheenhub_contracts::realtime::{DirectMessageCreated, TextChatMessage};
use dioxus::prelude::*;
use futures_util::StreamExt;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{Event, Notification, NotificationOptions};

use crate::Route;
use crate::features::app::active_room::ActiveRoomContext;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::{AudioPlaybackHandle, NotificationSound};
use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_ms;
use crate::features::social::realtime::subscribe_direct_message_events;
use crate::features::text_chat::realtime::{TextChatEvent, subscribe_text_chat};

use super::direct_messages::{keep_social_subscription_active, requires_attention};
use super::focus::ApplicationFocusContext;

/// Максимальная длина текста уведомления.
const MAX_NOTIFICATION_BODY_LEN: usize = 200;

/// Провайдер уведомлений о новых сообщениях чата и личных сообщений.
#[component]
pub(crate) fn NotificationsProvider(children: Element) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let current_user = use_context::<CurrentUserContext>();
    let active_room = use_context::<ActiveRoomContext>();
    let playback = use_context::<AudioPlaybackHandle>();
    let navigator = use_navigator();
    let mut pending_nav = use_signal(|| None::<Route>);
    let focus_state = use_signal(application_is_focused);
    use_context_provider(move || ApplicationFocusContext::new(focus_state));

    use_hook(move || {
        // Запрашиваем разрешение на уведомления при загрузке приложения.
        spawn(request_notification_permission());
        spawn(track_application_focus(focus_state));
        spawn(keep_social_subscription_active(realtime.clone()));

        // Подписываемся на события текстового чата и показываем уведомления
        // для сообщений, которые приходят не в активную комнату.
        let active_user_id = current_user.require_user().id.clone();
        spawn(listen_for_text_chat_messages(
            realtime.clone(),
            active_room,
            active_user_id.clone(),
            pending_nav,
        ));

        // Подписываемся на Social-события и показываем уведомления
        // для новых личных сообщений, которые приходят не в активный диалог.
        spawn(listen_for_dm_messages(
            realtime.clone(),
            active_room,
            playback,
            pending_nav,
        ));
    });

    // Реагируем на сигнал навигации от клика по уведомлению.
    use_effect(move || {
        let Some(route) = pending_nav() else {
            return;
        };
        pending_nav.set(None);
        debug!(route = %route, "navigating from notification click");
        navigator.push(route);
    });

    rsx! {
        {children}
    }
}

/// Отслеживает фокус вкладки, чтобы открытый диалог подтверждал прочтение после возврата.
async fn track_application_focus(mut focus_state: Signal<bool>) {
    loop {
        let next_focused = application_is_focused();
        if focus_state() != next_focused {
            focus_state.set(next_focused);
            debug!(next_focused, "updated browser application focus state");
        }
        sleep_ms(250).await;
    }
}

/// Запрашивает разрешение на браузерные уведомления.
async fn request_notification_permission() {
    let promise = match Notification::request_permission() {
        Ok(promise) => promise,
        Err(error) => {
            warn!(
                error = %error.as_string().unwrap_or_default(),
                "failed to request notification permission"
            );
            return;
        }
    };
    let permission = promise.await;
    match permission {
        Ok(perm) => {
            info!(permission = %perm.as_string().unwrap_or_default(), "notification permission resolved");
        }
        Err(error) => {
            warn!(
                error = %error.as_string().unwrap_or_default(),
                "notification permission promise rejected"
            );
        }
    }
}

/// Подписывается на события текстового чата и показывает уведомления
/// для сообщений, которые приходят не в активную комнату.
async fn listen_for_text_chat_messages(
    realtime: RealtimeHandle,
    active_room: ActiveRoomContext,
    active_user_id: String,
    pending_nav: Signal<Option<Route>>,
) {
    let mut receiver = subscribe_text_chat(&realtime);

    while let Some(event) = receiver.next().await {
        let TextChatEvent::MessageCreated(message) = event else {
            continue;
        };

        // Не показываем уведомления для собственных сообщений.
        if message.author_user_id == active_user_id {
            continue;
        }

        // Не показываем уведомления, если пользователь смотрит эту комнату.
        if active_room.get().as_deref() == Some(message.room_id.as_str()) {
            continue;
        }

        show_text_chat_notification(&message, &pending_nav);
    }
}

/// Подписывается на Social-события и показывает уведомления для новых
/// личных сообщений, которые приходят не в активный диалог.
async fn listen_for_dm_messages(
    realtime: RealtimeHandle,
    active_room: ActiveRoomContext,
    playback: AudioPlaybackHandle,
    pending_nav: Signal<Option<Route>>,
) {
    let mut receiver = subscribe_direct_message_events(&realtime);

    while let Some(notification) = receiver.next().await {
        let conversation_is_open =
            active_room.conversation_id().as_deref() == Some(notification.conversation_id.as_str());
        let application_is_focused = application_is_focused();
        let requires_attention = requires_attention(application_is_focused, conversation_is_open);
        if requires_attention {
            debug!(
                conversation_id = %notification.conversation_id,
                application_is_focused,
                conversation_is_open,
                "playing direct message notification sound"
            );
            playback.play_notification_sound(NotificationSound::MessageReceived);
        } else {
            debug!(
                conversation_id = %notification.conversation_id,
                "suppressed direct message notification sound for focused open conversation"
            );
        }

        if requires_attention {
            show_dm_notification(&notification, &pending_nav);
        }
    }
}

/// Проверяет, находится ли окно браузера и вкладка приложения в фокусе.
pub(crate) fn application_is_focused() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| !document.hidden() && document.has_focus().unwrap_or(false))
}

/// Создаёт браузерное уведомление о новом сообщении текстового чата
/// с навигацией при клике.
fn show_text_chat_notification(message: &TextChatMessage, pending_nav: &Signal<Option<Route>>) {
    let body = if message.body.is_empty() {
        "Отправил изображение".to_string()
    } else {
        truncate_message(&message.body)
    };

    let options = NotificationOptions::new();
    options.set_body(&body);

    let notification = match Notification::new_with_options(&message.author_nickname, &options) {
        Ok(notification) => notification,
        Err(error) => {
            warn!(
                error = %error.as_string().unwrap_or_default(),
                "failed to create notification"
            );
            return;
        }
    };

    let server_id = message.server_id.clone();
    let room_id = message.room_id.clone();
    let mut pending_nav = *pending_nav;

    let onclick = Closure::once(move |_event: Event| {
        // Фокусируем вкладку браузера.
        if let Some(window) = web_sys::window() {
            let _ = window.focus();
        }
        // Устанавливаем маршрут навигации; use_effect в провайдере обработает его.
        pending_nav.set(Some(Route::AppServerRoom { server_id, room_id }));
    });

    notification.set_onclick(Some(onclick.as_ref().unchecked_ref()));
    // Браузерное уведомление живет дольше текущего вызова Rust-функции.
    onclick.forget();
}

/// Создаёт браузерное уведомление о новом личном сообщении
/// с навигацией при клике.
fn show_dm_notification(data: &DirectMessageCreated, pending_nav: &Signal<Option<Route>>) {
    let body = if data.body.is_empty() {
        "Новое личное сообщение".to_string()
    } else {
        truncate_message(&data.body)
    };

    let options = NotificationOptions::new();
    options.set_body(&body);

    let notification = match Notification::new_with_options(&data.sender_nickname, &options) {
        Ok(notification) => notification,
        Err(error) => {
            warn!(
                error = %error.as_string().unwrap_or_default(),
                "failed to create DM notification"
            );
            return;
        }
    };

    let conversation_id = data.conversation_id.clone();
    let mut pending_nav = *pending_nav;

    let onclick = Closure::once(move |_event: Event| {
        // Фокусируем вкладку браузера.
        if let Some(window) = web_sys::window() {
            let _ = window.focus();
        }
        // Навигируем на личный диалог.
        pending_nav.set(Some(Route::AppDirectMessage {
            conversation_id: conversation_id.clone(),
        }));
    });

    notification.set_onclick(Some(onclick.as_ref().unchecked_ref()));
    // Браузерное уведомление живет дольше текущего вызова Rust-функции.
    onclick.forget();
}

/// Усекает сообщение до максимальной длины уведомления, сохраняя границы UTF-8.
fn truncate_message(body: &str) -> String {
    if body.len() <= MAX_NOTIFICATION_BODY_LEN {
        return body.to_string();
    }
    // Безопасное усечение по границам UTF-8 символов через итератор.
    // `char_indices` выдаёт точные границы символов, поэтому каждый индекс
    // из итератора гарантированно является char boundary.
    if let Some((i, _)) = body
        .char_indices()
        .rev()
        .find(|&(idx, _)| idx <= MAX_NOTIFICATION_BODY_LEN)
    {
        return format!("{}…", &body[..i]);
    }
    body.to_string()
}
