//! Android-интеграция системного уведомления о входящем личном звонке.

use dioxus::logger::tracing::warn;
use jni::objects::JValue;

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
