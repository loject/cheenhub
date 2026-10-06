//! Событийная подписка на onStart/onStop Android Activity без фонового polling.

use std::sync::{Mutex, OnceLock};

use dioxus::logger::tracing::{debug, warn};
use futures_channel::mpsc;
use futures_util::{StreamExt, stream::LocalBoxStream};
use jni::{JNIEnv, objects::JObject, sys::jboolean};

struct VisibilityState {
    visible: bool,
    subscribers: Vec<mpsc::UnboundedSender<bool>>,
}

fn visibility_state() -> &'static Mutex<VisibilityState> {
    static STATE: OnceLock<Mutex<VisibilityState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(VisibilityState {
            visible: true,
            subscribers: Vec::new(),
        })
    })
}

/// Регистрирует подписчика и отправляет текущую видимость Activity.
pub(super) fn subscribe_visibility() -> LocalBoxStream<'static, bool> {
    let (sender, receiver) = mpsc::unbounded();
    match visibility_state().lock() {
        Ok(mut state) => {
            let _ = sender.unbounded_send(state.visible);
            state
                .subscribers
                .retain(|subscriber| !subscriber.is_closed());
            state.subscribers.push(sender);
        }
        Err(error) => {
            warn!(%error, "Android application visibility state unavailable; suspending realtime");
            let _ = sender.unbounded_send(false);
        }
    }
    receiver.boxed_local()
}

/// Передаёт видимость Android Activity владельцу жизненного цикла realtime.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubVisibilityChanged(
    _env: JNIEnv<'_>,
    _activity: JObject<'_>,
    visible: jboolean,
) {
    let Ok(mut state) = visibility_state().lock() else {
        warn!("failed to update Android application visibility");
        return;
    };
    state.visible = visible != 0;
    let visible = state.visible;
    debug!(visible, "Android application visibility changed");
    state
        .subscribers
        .retain(|subscriber| subscriber.unbounded_send(visible).is_ok());
}
