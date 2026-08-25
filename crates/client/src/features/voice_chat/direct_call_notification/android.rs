//! Android-интеграция системного уведомления о входящем личном звонке.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use dioxus::logger::tracing::warn;
use futures_channel::{mpsc, oneshot};
use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use jni::sys::jint;

use super::IncomingCallNotificationAction;

const MAX_PENDING_ACTIONS: usize = 8;

static PENDING_ACTIONS: OnceLock<Mutex<VecDeque<IncomingCallNotificationAction>>> = OnceLock::new();
static ACTION_WAKEUP_SUBSCRIBERS: OnceLock<Mutex<Vec<mpsc::UnboundedSender<()>>>> = OnceLock::new();

/// Показывает системное уведомление о входящем звонке, когда приложение в фоне.
pub(crate) fn show_incoming_call_notification(
    call_id: String,
    conversation_id: String,
    caller_nickname: String,
) {
    wry::prelude::dispatch(move |env, activity, _| {
        let result = (|| {
            let call_id = env.new_string(call_id)?;
            let conversation_id = env.new_string(conversation_id)?;
            let caller_nickname = env.new_string(caller_nickname)?;
            env.call_method(
                activity,
                "showCheenHubIncomingCallNotification",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                &[
                    JValue::Object(&call_id),
                    JValue::Object(&conversation_id),
                    JValue::Object(&caller_nickname),
                ],
            )?;
            Ok::<_, jni::errors::Error>(())
        })();
        if let Err(error) = result {
            warn!(%error, "failed to show Android incoming-call notification");
        }
    });
}

/// Убирает системное уведомление после изменения состояния звонка.
pub(crate) fn clear_incoming_call_notification(call_id: String) {
    wry::prelude::dispatch(move |env, activity, _| {
        let result = env.new_string(call_id).and_then(|call_id| {
            env.call_method(
                activity,
                "clearCheenHubIncomingCallNotification",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&call_id)],
            )
        });
        if let Err(error) = result {
            warn!(%error, "failed to clear Android incoming-call notification");
        }
    });
}

/// Подписывается на сигнал появления действия из Android CallStyle.
pub(crate) fn subscribe_incoming_call_notification_action_wakeups() -> mpsc::UnboundedReceiver<()> {
    let (sender, receiver) = mpsc::unbounded();
    if let Ok(mut subscribers) = action_wakeup_subscribers().lock() {
        subscribers.push(sender);
    }
    receiver
}

/// Забирает следующее действие из живой JNI-очереди либо cold-start хранилища Activity.
pub(crate) async fn take_pending_incoming_call_notification_action()
-> Option<IncomingCallNotificationAction> {
    if let Ok(mut actions) = pending_actions().lock()
        && let Some(action) = actions.pop_front()
    {
        clear_persisted_action(action.call_id().to_owned());
        return Some(action);
    }

    let (sender, receiver) = oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let result = (|| -> Result<Option<IncomingCallNotificationAction>, jni::errors::Error> {
            let value = env
                .call_method(
                    activity,
                    "consumeCheenHubPendingDirectCallAction",
                    "()Ljava/lang/String;",
                    &[],
                )?
                .l()?;

            if value.is_null() {
                return Ok(None);
            }

            let value = JString::from(value);
            let value: String = env.get_string(&value)?.into();
            Ok(parse_persisted_action(&value))
        })();

        let _ = sender.send(result);
    });

    match receiver.await {
        Ok(Ok(action)) => action,
        Ok(Err(error)) => {
            warn!(%error, "failed to consume Android pending direct-call action");
            None
        }
        Err(_) => {
            warn!("Android pending direct-call action callback was dropped");
            None
        }
    }
}

fn clear_persisted_action(call_id: String) {
    wry::prelude::dispatch(move |env, activity, _| {
        let Ok(call_id) = env.new_string(call_id) else {
            return;
        };
        if let Err(error) = env.call_method(
            activity,
            "clearCheenHubPendingDirectCallAction",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&call_id)],
        ) {
            warn!(%error, "failed to clear persisted Android direct-call action");
        }
    });
}

fn parse_persisted_action(value: &str) -> Option<IncomingCallNotificationAction> {
    let (action, call_id) = value.split_once(':')?;
    if call_id.is_empty() {
        return None;
    }
    match action {
        "accept" => Some(IncomingCallNotificationAction::Accept(call_id.to_owned())),
        "decline" => Some(IncomingCallNotificationAction::Decline(call_id.to_owned())),
        _ => None,
    }
}

fn pending_actions() -> &'static Mutex<VecDeque<IncomingCallNotificationAction>> {
    PENDING_ACTIONS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn action_wakeup_subscribers() -> &'static Mutex<Vec<mpsc::UnboundedSender<()>>> {
    ACTION_WAKEUP_SUBSCRIBERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Принимает действие кнопки Android CallStyle.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubIncomingCallNotificationAction(
    mut env: JNIEnv<'_>,
    _activity: JObject<'_>,
    call_id: JString<'_>,
    action: jint,
) {
    let Ok(call_id) = env.get_string(&call_id).map(String::from) else {
        return;
    };

    let action = match action {
        1 => IncomingCallNotificationAction::Accept(call_id),
        2 => IncomingCallNotificationAction::Decline(call_id),
        _ => {
            warn!(
                action,
                "ignored unknown Android incoming-call notification action"
            );
            return;
        }
    };

    let Ok(mut actions) = pending_actions().lock() else {
        warn!("failed to lock Android incoming-call action queue");
        return;
    };

    while actions.len() >= MAX_PENDING_ACTIONS {
        actions.pop_front();
    }
    actions.push_back(action);
    drop(actions);

    if let Ok(mut subscribers) = action_wakeup_subscribers().lock() {
        subscribers.retain(|subscriber| subscriber.unbounded_send(()).is_ok());
    }
}
