//! Выбор платформенной интеграции уведомлений о личном звонке.

#[cfg(target_os = "android")]
#[path = "direct_call_notification/android.rs"]
mod implementation;

#[cfg(not(target_os = "android"))]
#[path = "direct_call_notification/unsupported.rs"]
mod implementation;

pub(crate) use implementation::{
    clear_incoming_call_notification, show_incoming_call_notification,
};
