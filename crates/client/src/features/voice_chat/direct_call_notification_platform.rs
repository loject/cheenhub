//! Выбор платформенной интеграции уведомлений о личном звонке.

/// Действие пользователя из системного Android-интерфейса входящего звонка.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum IncomingCallNotificationAction {
    Accept(String),
    Decline(String),
}

impl IncomingCallNotificationAction {
    pub(crate) fn call_id(&self) -> &str {
        match self {
            Self::Accept(call_id) | Self::Decline(call_id) => call_id,
        }
    }
}

#[cfg(target_os = "android")]
#[path = "direct_call_notification/android.rs"]
mod implementation;

#[cfg(not(target_os = "android"))]
#[path = "direct_call_notification/unsupported.rs"]
mod implementation;

pub(crate) use implementation::{
    clear_incoming_call_notification, show_incoming_call_notification,
    subscribe_incoming_call_notification_action_wakeups,
    take_pending_incoming_call_notification_action,
};
