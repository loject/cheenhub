//! Android-реализация реестра Activity/Service bridge.

#[cfg(target_os = "android")]
use std::sync::{Arc, OnceLock};

#[cfg(target_os = "android")]
mod callbacks;

#[cfg(target_os = "android")]
use super::{
    AndroidBridge, AndroidBridgeError, AndroidPermission, ForegroundServiceKind, guard_jni_result,
};
#[cfg(target_os = "android")]
use jni::objects::{JObject, JString, JValue};

#[cfg(target_os = "android")]
use callbacks::{
    PermissionCallback, ProjectionCallback, PushInstallationCallback, finish_permission_request,
    finish_projection_request, finish_push_installation_request, lock_error, next_request_id,
    permission_callbacks, projection_callbacks, push_installation_callbacks,
};

#[cfg(target_os = "android")]
pub(crate) use callbacks::take_media_projection_grant;

#[cfg(target_os = "android")]
static ANDROID_BRIDGE: OnceLock<Arc<dyn AndroidBridge>> = OnceLock::new();

#[cfg(target_os = "android")]
struct JniAndroidBridge;

#[cfg(target_os = "android")]
impl AndroidBridge for JniAndroidBridge {
    fn request_permission(
        &self,
        permission: AndroidPermission,
        callback: PermissionCallback,
    ) -> Result<(), AndroidBridgeError> {
        let request_id = next_request_id();
        permission_callbacks()
            .lock()
            .map_err(lock_error)?
            .insert(request_id, callback);
        if permission == AndroidPermission::PostNotifications {
            wry::prelude::dispatch(move |env, activity, _| {
                let result = env.call_method(
                    activity,
                    "requestCheenHubNotificationPermission",
                    "(I)V",
                    &[JValue::Int(request_id)],
                );
                if let Err(error) =
                    guard_jni_result(env, "requestCheenHubNotificationPermission", result)
                {
                    finish_permission_request(
                        request_id,
                        Err(AndroidBridgeError::new(format!(
                            "Не удалось запросить Android-разрешение уведомлений: {error}"
                        ))),
                    );
                }
            });
            return Ok(());
        }
        let permission = match permission {
            AndroidPermission::RecordAudio => "android.permission.RECORD_AUDIO",
            AndroidPermission::Camera => "android.permission.CAMERA",
            AndroidPermission::PostNotifications => unreachable!(),
        }
        .to_owned();
        wry::prelude::dispatch(move |env, activity, _| {
            let result = (|| -> Result<(), AndroidBridgeError> {
                let encoded_permission = env.new_string(permission);
                let permission = guard_jni_result(
                    env,
                    "requestCheenHubPermission.new_string",
                    encoded_permission,
                )
                .map_err(|error| {
                    AndroidBridgeError::new(format!(
                        "Не удалось подготовить Android-разрешение: {error}"
                    ))
                })?;
                let call = env.call_method(
                    activity,
                    "requestCheenHubPermission",
                    "(Ljava/lang/String;I)V",
                    &[JValue::Object(&permission), JValue::Int(request_id)],
                );
                guard_jni_result(env, "requestCheenHubPermission", call).map_err(|error| {
                    AndroidBridgeError::new(format!(
                        "Не удалось запросить Android-разрешение: {error}"
                    ))
                })?;
                Ok(())
            })();
            if let Err(error) = result {
                finish_permission_request(request_id, Err(error));
            }
        });
        Ok(())
    }

    fn request_media_projection(
        &self,
        callback: ProjectionCallback,
    ) -> Result<(), AndroidBridgeError> {
        let request_id = next_request_id();
        projection_callbacks()
            .lock()
            .map_err(lock_error)?
            .insert(request_id, callback);
        wry::prelude::dispatch(move |env, activity, _| {
            let call = env.call_method(
                activity,
                "requestCheenHubMediaProjection",
                "(I)V",
                &[JValue::Int(request_id)],
            );
            if let Err(error) = guard_jni_result(env, "requestCheenHubMediaProjection", call) {
                finish_projection_request(
                    request_id,
                    Err(AndroidBridgeError::new(format!(
                        "Не удалось открыть Android MediaProjection: {error}"
                    ))),
                );
            }
        });
        Ok(())
    }

    fn start_foreground_service(
        &self,
        kind: ForegroundServiceKind,
    ) -> Result<(), AndroidBridgeError> {
        dispatch_service("startCheenHubForegroundService", kind);
        Ok(())
    }

    fn stop_foreground_service(
        &self,
        kind: ForegroundServiceKind,
    ) -> Result<(), AndroidBridgeError> {
        dispatch_service("stopCheenHubForegroundService", kind);
        Ok(())
    }

    fn request_push_installation(
        &self,
        callback: PushInstallationCallback,
    ) -> Result<(), AndroidBridgeError> {
        let request_id = next_request_id();
        push_installation_callbacks()
            .lock()
            .map_err(lock_error)?
            .insert(request_id, callback);
        wry::prelude::dispatch(move |env, activity, _| {
            let call = env.call_method(
                activity,
                "requestCheenHubPushInstallation",
                "(I)V",
                &[JValue::Int(request_id)],
            );
            if let Err(error) = guard_jni_result(env, "requestCheenHubPushInstallation", call) {
                finish_push_installation_request(
                    request_id,
                    Err(AndroidBridgeError::new(format!(
                        "Не удалось получить Android push installation: {error}"
                    ))),
                );
            }
        });
        Ok(())
    }

    fn take_pending_direct_message_conversation_id(
        &self,
        callback: Box<dyn FnOnce(Result<Option<String>, AndroidBridgeError>) + Send>,
    ) -> Result<(), AndroidBridgeError> {
        wry::prelude::dispatch(move |env, activity, _| {
            let call = env.call_method(
                activity,
                "consumeCheenHubPendingDirectMessageConversationId",
                "()Ljava/lang/String;",
                &[],
            );
            let result = guard_jni_result(
                env,
                "consumeCheenHubPendingDirectMessageConversationId",
                call,
            )
            .and_then(|value| value.l())
            .map_err(|error| {
                AndroidBridgeError::new(format!(
                    "Не удалось получить переход из Android-уведомления: {error}"
                ))
            })
            .and_then(|value| {
                if value.is_null() {
                    Ok(None)
                } else {
                    let value = JString::from(value);
                    env.get_string(&value)
                        .map(|value| Some(value.into()))
                        .map_err(|error| {
                            AndroidBridgeError::new(format!(
                                "Не удалось прочитать идентификатор диалога: {error}"
                            ))
                        })
                }
            });
            callback(result);
        });
        Ok(())
    }

    fn take_pending_friend_requests(
        &self,
        callback: Box<dyn FnOnce(Result<bool, AndroidBridgeError>) + Send>,
    ) -> Result<(), AndroidBridgeError> {
        wry::prelude::dispatch(move |env, activity, _| {
            let call =
                env.call_method(activity, "consumeCheenHubPendingFriendRequests", "()Z", &[]);
            let result = guard_jni_result(env, "consumeCheenHubPendingFriendRequests", call)
                .and_then(|value| value.z())
                .map_err(|error| {
                    AndroidBridgeError::new(format!(
                        "Не удалось получить переход к заявкам в друзья: {error}"
                    ))
                });
            callback(result);
        });
        Ok(())
    }

    fn set_active_direct_message_conversation(
        &self,
        conversation_id: Option<String>,
    ) -> Result<(), AndroidBridgeError> {
        wry::prelude::dispatch(move |env, activity, _| match conversation_id {
            Some(conversation_id) => {
                let encoded = env.new_string(conversation_id);
                if let Ok(conversation_id) = guard_jni_result(
                    env,
                    "setCheenHubActiveDirectMessageConversationId.new_string",
                    encoded,
                ) {
                    let result = env.call_method(
                        activity,
                        "setCheenHubActiveDirectMessageConversationId",
                        "(Ljava/lang/String;)V",
                        &[JValue::Object(&conversation_id)],
                    );
                    let _ = guard_jni_result(
                        env,
                        "setCheenHubActiveDirectMessageConversationId",
                        result,
                    );
                }
            }
            None => {
                let null = JObject::null();
                let result = env.call_method(
                    activity,
                    "setCheenHubActiveDirectMessageConversationId",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&null)],
                );
                let _ =
                    guard_jni_result(env, "setCheenHubActiveDirectMessageConversationId", result);
            }
        });
        Ok(())
    }

    fn clear_direct_message_notification(
        &self,
        conversation_id: String,
    ) -> Result<(), AndroidBridgeError> {
        wry::prelude::dispatch(move |env, activity, _| {
            let encoded = env.new_string(conversation_id);
            if let Ok(conversation_id) = guard_jni_result(
                env,
                "clearCheenHubDirectMessageNotification.new_string",
                encoded,
            ) {
                let result = env.call_method(
                    activity,
                    "clearCheenHubDirectMessageNotification",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&conversation_id)],
                );
                let _ = guard_jni_result(env, "clearCheenHubDirectMessageNotification", result);
            }
        });
        Ok(())
    }
}

/// Возвращает установленный Android Activity/Service bridge.
#[cfg(target_os = "android")]
pub(crate) fn android_bridge() -> Result<&'static Arc<dyn AndroidBridge>, AndroidBridgeError> {
    Ok(ANDROID_BRIDGE.get_or_init(|| Arc::new(JniAndroidBridge)))
}

#[cfg(target_os = "android")]
fn dispatch_service(method: &'static str, kind: ForegroundServiceKind) {
    let kind = match kind {
        ForegroundServiceKind::VoicePlayback => "voicePlayback",
        ForegroundServiceKind::Microphone => "microphone",
        ForegroundServiceKind::Camera => "camera",
        ForegroundServiceKind::MediaProjection => "mediaProjection",
    }
    .to_owned();
    wry::prelude::dispatch(move |env, activity, _| {
        let encoded = env.new_string(kind);
        if let Ok(kind) = guard_jni_result(env, "foregroundService.new_string", encoded) {
            let result = env.call_method(
                activity,
                method,
                "(Ljava/lang/String;)V",
                &[JValue::Object(&kind)],
            );
            let _ = guard_jni_result(env, method, result);
        }
    });
}
