//! Web-реализация уведомлений о сообщениях.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use cheenhub_contracts::realtime::{SocialChangeReason, SocialChanged, TextChatMessage};
use dioxus::prelude::*;
use futures_util::StreamExt;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{Event, Notification, NotificationOptions};

use crate::Route;
use crate::features::app::active_room::ActiveRoomContext;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::realtime::RealtimeHandle;
use crate::features::social::api;
use crate::features::social::realtime::subscribe_social_events;
use crate::features::text_chat::realtime::{TextChatEvent, subscribe_text_chat};

/// Максимальная длина текста уведомления.
const MAX_NOTIFICATION_BODY_LEN: usize = 200;

/// Провайдер уведомлений о новых сообщениях чата и личных сообщений.
#[component]
pub(crate) fn NotificationsProvider(children: Element) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let current_user = use_context::<CurrentUserContext>();
    let active_room = use_context::<ActiveRoomContext>();
    let navigator = use_navigator();
    let mut pending_nav = use_signal(|| None::<Route>);
    let dm_unread_snapshot = use_signal(Vec::<(String, i64)>::new);

    use_hook(move || {
        // Запрашиваем разрешение на уведомления при загрузке приложения.
        spawn(request_notification_permission());
        spawn(load_initial_dm_unread_snapshot(dm_unread_snapshot));

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
            active_user_id,
            pending_nav,
            dm_unread_snapshot,
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
///
/// Поскольку `SocialChanged` содержит только `conversation_id` без деталей
/// сообщения, при получении события мы загружаем список диалогов, находим
/// диалог с непрочитанными сообщениями и загружаем его сообщения для
/// извлечения данных последнего сообщения.
async fn listen_for_dm_messages(
    realtime: RealtimeHandle,
    active_room: ActiveRoomContext,
    active_user_id: String,
    pending_nav: Signal<Option<Route>>,
    dm_unread_snapshot: Signal<Vec<(String, i64)>>,
) {
    let mut receiver = subscribe_social_events(&realtime);

    while let Some(event) = receiver.next().await {
        // Нас интересуют только изменения личных сообщений.
        if event.reason != SocialChangeReason::DirectMessages {
            continue;
        }

        let dm_notification =
            extract_dm_notification(&event, &active_room, &active_user_id, dm_unread_snapshot)
                .await;

        if let Some(notification) = dm_notification {
            show_dm_notification(&notification, &pending_nav);
        }
    }
}

/// Извлекает данные для уведомления о новом личном сообщении.
///
/// Возвращает `None`, если:
/// - Диалог уже активен (пользователь его смотрит).
/// - Не удалось загрузить данные диалогов или сообщений.
/// - Новое сообщение отправил текущий пользователь.
async fn extract_dm_notification(
    event: &SocialChanged,
    active_room: &ActiveRoomContext,
    active_user_id: &str,
    mut dm_unread_snapshot: Signal<Vec<(String, i64)>>,
) -> Option<DmNotificationData> {
    // Если событие привязано к конкретному диалогу и это активный диалог,
    // не показываем уведомление.
    if let Some(ref conv_id) = event.conversation_id
        && active_room.conversation_id().as_deref() == Some(conv_id.as_str())
    {
        return None;
    }

    // Загружаем актуальный список диалогов.
    let conversations = match api::list_dm_conversations().await {
        Ok(convs) => convs,
        Err(err) => {
            debug!(%err, "failed to load DM conversations for notification");
            return None;
        }
    };

    let previous_snapshot = dm_unread_snapshot();
    let next_snapshot = conversations
        .iter()
        .map(|conversation| (conversation.id.clone(), conversation.unread_count))
        .collect::<Vec<_>>();

    // Находим диалог с непрочитанными сообщениями.
    // Если событие указывает конкретный диалог, приоритизируем его.
    let target_conversation = if let Some(ref conv_id) = event.conversation_id {
        conversations.iter().find(|c| c.id == *conv_id)
    } else {
        conversations.iter().find(|c| c.unread_count > 0)
    };

    let conversation = match target_conversation {
        Some(c) => c.clone(),
        None => {
            dm_unread_snapshot.set(next_snapshot);
            return None;
        }
    };

    let previous_unread = unread_count_for(&previous_snapshot, &conversation.id);
    let unread_increased = previous_unread
        .map(|unread_count| conversation.unread_count > unread_count)
        .unwrap_or(false);
    dm_unread_snapshot.set(next_snapshot);

    if !unread_increased {
        debug!(
            conversation_id = %conversation.id,
            unread_count = conversation.unread_count,
            previous_unread = previous_unread.unwrap_or_default(),
            "skipping direct message notification without unread increase"
        );
        return None;
    }

    // Загружаем сообщения этого диалога для получения последнего сообщения.
    let messages = match api::list_dm_messages(&conversation.id).await {
        Ok(resp) => resp.messages,
        Err(err) => {
            debug!(%err, conversation_id = %conversation.id, "failed to load DM messages for notification");
            return None;
        }
    };

    // Находим последнее непрочитанное сообщение, не от текущего пользователя.
    let last_message = messages
        .into_iter()
        .rev()
        .find(|msg| msg.sender_user_id != active_user_id);

    let message = match last_message {
        Some(m) => m,
        None => return None,
    };

    Some(DmNotificationData {
        conversation_id: conversation.id,
        sender_nickname: message.sender_nickname,
        body: message.body,
    })
}

/// Загружает начальный снимок непрочитанных ЛС, чтобы не показывать старые
/// уведомления при открытии приложения или страницы друзей.
async fn load_initial_dm_unread_snapshot(mut dm_unread_snapshot: Signal<Vec<(String, i64)>>) {
    match api::list_dm_conversations().await {
        Ok(conversations) => {
            let snapshot = conversations
                .into_iter()
                .map(|conversation| (conversation.id, conversation.unread_count))
                .collect::<Vec<_>>();
            dm_unread_snapshot.set(snapshot);
            debug!("loaded initial direct message unread snapshot for notifications");
        }
        Err(err) => {
            debug!(%err, "failed to load initial DM unread snapshot for notifications");
        }
    }
}

fn unread_count_for(snapshot: &[(String, i64)], conversation_id: &str) -> Option<i64> {
    snapshot
        .iter()
        .find_map(|(saved_conversation_id, unread_count)| {
            (saved_conversation_id == conversation_id).then_some(*unread_count)
        })
}

/// Данные для уведомления о личном сообщении.
struct DmNotificationData {
    conversation_id: String,
    sender_nickname: String,
    body: String,
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
fn show_dm_notification(data: &DmNotificationData, pending_nav: &Signal<Option<Route>>) {
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
