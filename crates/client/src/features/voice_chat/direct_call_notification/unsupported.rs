//! Заглушка системного уведомления для платформ без Android-интеграции.

/// На этой платформе отдельное системное уведомление не требуется.
pub(crate) fn show_incoming_call_notification(
    _call_id: String,
    _conversation_id: String,
    _caller_nickname: String,
) {
}

/// На этой платформе отдельное системное уведомление не показывается.
pub(crate) fn clear_incoming_call_notification(_call_id: String) {}
