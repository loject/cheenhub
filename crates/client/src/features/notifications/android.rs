//! Android-реализация регистрации системных push-уведомлений.
#![cfg_attr(not(target_os = "android"), allow(dead_code, unused_imports))]

#[cfg(target_os = "android")]
mod api;

#[cfg(target_os = "android")]
use std::sync::{Mutex, OnceLock};

use dioxus::prelude::*;
#[cfg(target_os = "android")]
use futures_channel::{mpsc, oneshot};
#[cfg(target_os = "android")]
use futures_util::StreamExt;
#[cfg(target_os = "android")]
use jni::JNIEnv;
#[cfg(target_os = "android")]
use jni::objects::{JObject, JString};

use crate::Route;
#[cfg(target_os = "android")]
use crate::features::app::active_room::ActiveRoomContext;
#[cfg(target_os = "android")]
use crate::features::runtime::android::{
    AndroidPermission, AndroidPushInstallation, PermissionResult, android_bridge,
};

#[cfg(target_os = "android")]
static NOTIFICATION_OPEN_SUBSCRIBERS: OnceLock<Mutex<Vec<mpsc::UnboundedSender<String>>>> =
    OnceLock::new();
#[cfg(target_os = "android")]
static FRIEND_REQUEST_OPEN_SUBSCRIBERS: OnceLock<Mutex<Vec<mpsc::UnboundedSender<()>>>> =
    OnceLock::new();

/// Регистрирует Android push-установку и открывает экран из нажатого уведомления.
///
/// Обычное восстановление сохранённой рабочей области остаётся отдельной политикой запуска.
#[component]
pub(crate) fn NotificationsProvider(children: Element) -> Element {
    #[cfg(target_os = "android")]
    {
        let active_room = use_context::<ActiveRoomContext>();
        let navigator = use_navigator();
        let mut pending_route = use_signal(|| None::<Route>);

        use_hook(move || {
            spawn(register_android_installation());
            spawn(async move {
                let mut opened = subscribe_notification_opens();
                match take_pending_conversation().await {
                    Ok(Some(conversation_id)) => {
                        info!(
                            %conversation_id,
                            source = "cold_start",
                            "queued Android direct-message notification route"
                        );
                        pending_route.set(Some(direct_message_route(conversation_id)));
                    }
                    Ok(None) => {}
                    Err(error) => warn!(
                        %error,
                        "failed to consume pending Android notification route"
                    ),
                }
                while let Some(conversation_id) = opened.next().await {
                    // Intent хранится и для cold start. При живом callback удаляем
                    // сохранённое значение, чтобы следующий mount не открыл его повторно.
                    if let Err(error) = take_pending_conversation().await {
                        warn!(
                            %error,
                            "failed to clear delivered Android notification route"
                        );
                    }
                    info!(
                        %conversation_id,
                        source = "activity_intent",
                        "queued Android direct-message notification route"
                    );
                    pending_route.set(Some(direct_message_route(conversation_id)));
                }
                warn!("Android notification-open subscription stopped");
            });
            spawn(async move {
                match take_pending_friend_requests().await {
                    Ok(true) => {
                        info!(
                            source = "cold_start",
                            "queued Android friend-request notification route"
                        );
                        pending_route.set(Some(friend_requests_route()));
                    }
                    Ok(false) => {}
                    Err(error) => warn!(
                        %error,
                        "failed to consume pending Android friend-request route"
                    ),
                }
                let mut opened = subscribe_friend_request_opens();
                while opened.next().await.is_some() {
                    if let Err(error) = take_pending_friend_requests().await {
                        warn!(
                            %error,
                            "failed to clear delivered Android friend-request route"
                        );
                    }
                    info!(
                        source = "activity_intent",
                        "queued Android friend-request notification route"
                    );
                    pending_route.set(Some(friend_requests_route()));
                }
                warn!("Android friend-request notification-open subscription stopped");
            });
        });

        use_effect(move || {
            let Some(route) = pending_route() else {
                return;
            };
            pending_route.set(None);
            info!(route = %route, "navigating from Android notification click");
            navigator.push(route);
        });

        use_effect(move || {
            let conversation_id = active_room.conversation_id();
            let Ok(bridge) = android_bridge() else {
                return;
            };
            if let Err(error) =
                bridge.set_active_direct_message_conversation(conversation_id.clone())
            {
                warn!(%error, "failed to update active Android direct conversation");
            }
            if let Some(conversation_id) = conversation_id
                && let Err(error) = bridge.clear_direct_message_notification(conversation_id)
            {
                warn!(%error, "failed to clear opened Android direct-message notification");
            }
        });
    }

    rsx! { {children} }
}

fn direct_message_route(conversation_id: String) -> Route {
    Route::AppDirectMessage { conversation_id }
}

fn friend_requests_route() -> Route {
    Route::AppFriends {}
}

#[cfg(target_os = "android")]
async fn register_android_installation() {
    match request_notification_permission().await {
        Ok(PermissionResult::Granted) => {}
        Ok(PermissionResult::Denied | PermissionResult::DeniedPermanently) => {
            info!("Android notification permission was not granted");
            return;
        }
        Err(error) => {
            warn!(%error, "failed to request Android notification permission");
            return;
        }
    }
    let installation = match request_push_installation().await {
        Ok(installation) => installation,
        Err(error) => {
            warn!(%error, "failed to load Android push installation");
            return;
        }
    };
    let installation_id = installation.installation_id.clone();
    match api::upsert_installation(&installation_id, installation.token).await {
        Ok(()) => info!(%installation_id, "registered Android push installation"),
        Err(error) => {
            warn!(%installation_id, %error, "failed to register Android push installation")
        }
    }
}

#[cfg(target_os = "android")]
async fn request_notification_permission() -> Result<PermissionResult, String> {
    let (sender, receiver) = oneshot::channel();
    android_bridge()
        .map_err(|error| error.to_string())?
        .request_permission(
            AndroidPermission::PostNotifications,
            Box::new(move |result| {
                let _ = sender.send(result.map_err(|error| error.to_string()));
            }),
        )
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "Android закрыл callback разрешения уведомлений.".to_owned())?
}

#[cfg(target_os = "android")]
async fn request_push_installation() -> Result<AndroidPushInstallation, String> {
    let (sender, receiver) = oneshot::channel();
    android_bridge()
        .map_err(|error| error.to_string())?
        .request_push_installation(Box::new(move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        }))
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "Android закрыл callback FCM-установки.".to_owned())?
}

#[cfg(target_os = "android")]
async fn take_pending_conversation() -> Result<Option<String>, String> {
    let (sender, receiver) = oneshot::channel();
    android_bridge()
        .map_err(|error| error.to_string())?
        .take_pending_direct_message_conversation_id(Box::new(move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        }))
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "Android закрыл callback маршрута уведомления.".to_owned())?
}

#[cfg(target_os = "android")]
async fn take_pending_friend_requests() -> Result<bool, String> {
    let (sender, receiver) = oneshot::channel();
    android_bridge()
        .map_err(|error| error.to_string())?
        .take_pending_friend_requests(Box::new(move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        }))
        .map_err(|error| error.to_string())?;
    receiver
        .await
        .map_err(|_| "Android закрыл callback маршрута заявок в друзья.".to_owned())?
}

#[cfg(target_os = "android")]
fn subscribe_notification_opens() -> mpsc::UnboundedReceiver<String> {
    let (sender, receiver) = mpsc::unbounded();
    if let Ok(mut subscribers) = notification_open_subscribers().lock() {
        subscribers.push(sender);
    }
    receiver
}

#[cfg(target_os = "android")]
fn notification_open_subscribers() -> &'static Mutex<Vec<mpsc::UnboundedSender<String>>> {
    NOTIFICATION_OPEN_SUBSCRIBERS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(target_os = "android")]
fn subscribe_friend_request_opens() -> mpsc::UnboundedReceiver<()> {
    let (sender, receiver) = mpsc::unbounded();
    if let Ok(mut subscribers) = friend_request_open_subscribers().lock() {
        subscribers.push(sender);
    }
    receiver
}

#[cfg(target_os = "android")]
fn friend_request_open_subscribers() -> &'static Mutex<Vec<mpsc::UnboundedSender<()>>> {
    FRIEND_REQUEST_OPEN_SUBSCRIBERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Передаёт открытие Android-уведомления активному Dioxus provider.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubDirectMessageNotificationOpened(
    mut env: JNIEnv<'_>,
    _activity: JObject<'_>,
    conversation_id: JString<'_>,
) {
    let Ok(conversation_id) = env.get_string(&conversation_id).map(String::from) else {
        return;
    };
    let Ok(mut subscribers) = notification_open_subscribers().lock() else {
        return;
    };
    subscribers.retain(|subscriber| subscriber.unbounded_send(conversation_id.clone()).is_ok());
}

/// Передаёт открытие Android-уведомления о заявке в друзья активному Dioxus provider.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubFriendRequestNotificationOpened(
    _env: JNIEnv<'_>,
    _activity: JObject<'_>,
) {
    let Ok(mut subscribers) = friend_request_open_subscribers().lock() else {
        return;
    };
    subscribers.retain(|subscriber| subscriber.unbounded_send(()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_click_targets_direct_message_route() {
        let conversation_id = "80c993e1-2fe7-49e0-bcc5-c56c790d98c8".to_owned();

        assert_eq!(
            direct_message_route(conversation_id.clone()),
            Route::AppDirectMessage { conversation_id }
        );
    }

    #[test]
    fn friend_request_notification_click_targets_friends_route() {
        assert_eq!(friend_requests_route(), Route::AppFriends {});
    }
}
