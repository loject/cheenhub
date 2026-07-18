//! Android-реализация регистрации и навигации системных push-уведомлений.
#![cfg_attr(not(target_os = "android"), allow(dead_code, unused_imports))]

#[cfg(target_os = "android")]
mod api;

#[cfg(target_os = "android")]
use std::sync::{Mutex, OnceLock};

use dioxus::prelude::*;
#[cfg(target_os = "android")]
use dioxus::router::Navigator;
#[cfg(target_os = "android")]
use futures_channel::{mpsc, oneshot};
#[cfg(target_os = "android")]
use futures_util::StreamExt;
#[cfg(target_os = "android")]
use jni::JNIEnv;
#[cfg(target_os = "android")]
use jni::objects::{JObject, JString};

#[cfg(target_os = "android")]
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

/// Регистрирует Android push-установку и связывает notification click с маршрутом ЛС.
#[component]
pub(crate) fn NotificationsProvider(children: Element) -> Element {
    #[cfg(target_os = "android")]
    {
        let active_room = use_context::<ActiveRoomContext>();
        let navigator = use_navigator();

        use_hook(move || {
            spawn(register_android_installation());
            let navigator = navigator.clone();
            spawn(async move {
                let mut opened = subscribe_notification_opens();
                if let Ok(Some(conversation_id)) = take_pending_conversation().await {
                    navigate_to_conversation(&navigator, conversation_id);
                }
                while let Some(conversation_id) = opened.next().await {
                    // Intent хранится и для cold start. При живом callback сразу потребляем
                    // сохранённый маршрут, чтобы следующий mount не открыл тот же диалог повторно.
                    let _ = take_pending_conversation().await;
                    navigate_to_conversation(&navigator, conversation_id);
                }
                warn!("Android notification-open subscription stopped");
            });
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
fn navigate_to_conversation(navigator: &Navigator, conversation_id: String) {
    debug!(%conversation_id, "opening direct conversation from Android notification");
    navigator.push(Route::AppDirectMessage { conversation_id });
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
