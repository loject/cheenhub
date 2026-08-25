//! Заглушка системного уведомления для платформ без Android-интеграции.

use futures_channel::mpsc;

use super::IncomingCallNotificationAction;

/// На этой платформе отдельное системное уведомление не требуется.
pub(crate) fn show_incoming_call_notification(
    _call_id: String,
    _conversation_id: String,
    _caller_nickname: String,
) {
}

/// На этой платформе отдельное системное уведомление не показывается.
pub(crate) fn clear_incoming_call_notification(_call_id: String) {}

/// На не-Android платформах native call actions отсутствуют.
pub(crate) fn subscribe_incoming_call_notification_action_wakeups() -> mpsc::UnboundedReceiver<()> {
    let (_sender, receiver) = mpsc::unbounded();
    receiver
}

/// На не-Android платформах pending native action отсутствует.
pub(crate) async fn take_pending_incoming_call_notification_action()
-> Option<IncomingCallNotificationAction> {
    None
}
