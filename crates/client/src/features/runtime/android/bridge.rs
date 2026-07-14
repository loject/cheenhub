//! Android-реализация реестра Activity/Service bridge.

#[cfg(target_os = "android")]
use std::collections::HashMap;
#[cfg(target_os = "android")]
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(target_os = "android")]
use super::{
    AndroidBridge, AndroidBridgeError, AndroidPermission, AndroidPushInstallation,
    ForegroundServiceKind, MediaProjectionGrant, PermissionResult,
};
#[cfg(target_os = "android")]
use jni::JNIEnv;
#[cfg(target_os = "android")]
use jni::objects::{GlobalRef, JObject, JString, JValue};
#[cfg(target_os = "android")]
use jni::sys::{jboolean, jint, jstring};

#[cfg(target_os = "android")]
static ANDROID_BRIDGE: OnceLock<Arc<dyn AndroidBridge>> = OnceLock::new();

#[cfg(target_os = "android")]
type PermissionCallback =
    Box<dyn FnOnce(Result<PermissionResult, AndroidBridgeError>) + Send + 'static>;
#[cfg(target_os = "android")]
type ProjectionCallback =
    Box<dyn FnOnce(Result<Option<MediaProjectionGrant>, AndroidBridgeError>) + Send + 'static>;
#[cfg(target_os = "android")]
type PushInstallationCallback =
    Box<dyn FnOnce(Result<AndroidPushInstallation, AndroidBridgeError>) + Send + 'static>;

#[cfg(target_os = "android")]
static PERMISSION_CALLBACKS: OnceLock<Mutex<HashMap<i32, PermissionCallback>>> = OnceLock::new();
#[cfg(target_os = "android")]
static PROJECTION_CALLBACKS: OnceLock<Mutex<HashMap<i32, ProjectionCallback>>> = OnceLock::new();
#[cfg(target_os = "android")]
static PUSH_INSTALLATION_CALLBACKS: OnceLock<Mutex<HashMap<i32, PushInstallationCallback>>> =
    OnceLock::new();
#[cfg(target_os = "android")]
static PROJECTION_GRANTS: OnceLock<Mutex<HashMap<u64, GlobalRef>>> = OnceLock::new();
#[cfg(target_os = "android")]
static NEXT_REQUEST_ID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1000);
#[cfg(target_os = "android")]
static NEXT_GRANT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

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
                let _ = env.call_method(
                    activity,
                    "requestCheenHubNotificationPermission",
                    "(I)V",
                    &[JValue::Int(request_id)],
                );
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
            if let Ok(permission) = env.new_string(permission) {
                let _ = env.call_method(
                    activity,
                    "requestCheenHubPermission",
                    "(Ljava/lang/String;I)V",
                    &[JValue::Object(&permission), JValue::Int(request_id)],
                );
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
            let _ = env.call_method(
                activity,
                "requestCheenHubMediaProjection",
                "(I)V",
                &[JValue::Int(request_id)],
            );
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
            let _ = env.call_method(
                activity,
                "requestCheenHubPushInstallation",
                "(I)V",
                &[JValue::Int(request_id)],
            );
        });
        Ok(())
    }

    fn take_pending_direct_message_conversation_id(
        &self,
        callback: Box<dyn FnOnce(Result<Option<String>, AndroidBridgeError>) + Send>,
    ) -> Result<(), AndroidBridgeError> {
        wry::prelude::dispatch(move |env, activity, _| {
            let result = env
                .call_method(
                    activity,
                    "consumeCheenHubPendingDirectMessageConversationId",
                    "()Ljava/lang/String;",
                    &[],
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

    fn set_active_direct_message_conversation(
        &self,
        conversation_id: Option<String>,
    ) -> Result<(), AndroidBridgeError> {
        wry::prelude::dispatch(move |env, activity, _| match conversation_id {
            Some(conversation_id) => {
                if let Ok(conversation_id) = env.new_string(conversation_id) {
                    let _ = env.call_method(
                        activity,
                        "setCheenHubActiveDirectMessageConversationId",
                        "(Ljava/lang/String;)V",
                        &[JValue::Object(&conversation_id)],
                    );
                }
            }
            None => {
                let null = JObject::null();
                let _ = env.call_method(
                    activity,
                    "setCheenHubActiveDirectMessageConversationId",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&null)],
                );
            }
        });
        Ok(())
    }

    fn clear_direct_message_notification(
        &self,
        conversation_id: String,
    ) -> Result<(), AndroidBridgeError> {
        wry::prelude::dispatch(move |env, activity, _| {
            if let Ok(conversation_id) = env.new_string(conversation_id) {
                let _ = env.call_method(
                    activity,
                    "clearCheenHubDirectMessageNotification",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&conversation_id)],
                );
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
pub(crate) fn take_media_projection_grant(
    grant: MediaProjectionGrant,
) -> Result<GlobalRef, AndroidBridgeError> {
    projection_grants()
        .lock()
        .map_err(lock_error)?
        .remove(&grant.0)
        .ok_or_else(|| {
            AndroidBridgeError::new("MediaProjection grant отсутствует или уже использован")
        })
}

#[cfg(target_os = "android")]
fn dispatch_service(method: &'static str, kind: ForegroundServiceKind) {
    let kind = match kind {
        ForegroundServiceKind::Voice => "voice",
        ForegroundServiceKind::Camera => "camera",
        ForegroundServiceKind::MediaProjection => "mediaProjection",
    }
    .to_owned();
    wry::prelude::dispatch(move |env, activity, _| {
        if let Ok(kind) = env.new_string(kind) {
            let _ = env.call_method(
                activity,
                method,
                "(Ljava/lang/String;)V",
                &[JValue::Object(&kind)],
            );
        }
    });
}

#[cfg(target_os = "android")]
fn next_request_id() -> i32 {
    NEXT_REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(target_os = "android")]
fn permission_callbacks() -> &'static Mutex<HashMap<i32, PermissionCallback>> {
    PERMISSION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "android")]
fn projection_callbacks() -> &'static Mutex<HashMap<i32, ProjectionCallback>> {
    PROJECTION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "android")]
fn push_installation_callbacks() -> &'static Mutex<HashMap<i32, PushInstallationCallback>> {
    PUSH_INSTALLATION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "android")]
fn projection_grants() -> &'static Mutex<HashMap<u64, GlobalRef>> {
    PROJECTION_GRANTS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "android")]
fn lock_error<T>(_error: std::sync::PoisonError<T>) -> AndroidBridgeError {
    AndroidBridgeError::new("Android bridge state повреждён")
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubPermissionResult(
    _env: JNIEnv<'_>,
    _activity: JObject<'_>,
    request_id: jint,
    granted: jboolean,
    can_ask_again: jboolean,
) {
    let callback = permission_callbacks()
        .lock()
        .ok()
        .and_then(|mut callbacks| callbacks.remove(&request_id));
    if let Some(callback) = callback {
        callback(Ok(if granted != 0 {
            PermissionResult::Granted
        } else if can_ask_again != 0 {
            PermissionResult::Denied
        } else {
            PermissionResult::DeniedPermanently
        }));
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubMediaProjectionResult(
    env: JNIEnv<'_>,
    _activity: JObject<'_>,
    request_id: jint,
    granted: jboolean,
    data: JObject<'_>,
) {
    let callback = projection_callbacks()
        .lock()
        .ok()
        .and_then(|mut callbacks| callbacks.remove(&request_id));
    if let Some(callback) = callback {
        if granted == 0 || data.is_null() {
            callback(Ok(None));
            return;
        }
        match env.new_global_ref(data) {
            Ok(data) => {
                let id = NEXT_GRANT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if let Ok(mut grants) = projection_grants().lock() {
                    grants.insert(id, data);
                    callback(Ok(Some(MediaProjectionGrant(id))));
                } else {
                    callback(Err(AndroidBridgeError::new(
                        "Не удалось сохранить MediaProjection grant",
                    )));
                }
            }
            Err(error) => callback(Err(AndroidBridgeError::new(format!(
                "Не удалось сохранить MediaProjection Intent: {error}"
            )))),
        }
    }
}

/// Завершает асинхронное получение Android push-установки.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubPushInstallationResult(
    mut env: JNIEnv<'_>,
    _activity: JObject<'_>,
    request_id: jint,
    installation_id: jstring,
    token: jstring,
    error_code: jstring,
) {
    let callback = push_installation_callbacks()
        .lock()
        .ok()
        .and_then(|mut callbacks| callbacks.remove(&request_id));
    let Some(callback) = callback else {
        return;
    };
    if let Some(error_code) = optional_java_string(&mut env, error_code) {
        callback(Err(AndroidBridgeError::new(format!(
            "Android push installation недоступна: {error_code}"
        ))));
        return;
    }
    let installation_id = optional_java_string(&mut env, installation_id);
    let token = optional_java_string(&mut env, token);
    match (installation_id, token) {
        (Some(installation_id), Some(token)) => callback(Ok(AndroidPushInstallation {
            installation_id,
            token,
        })),
        _ => callback(Err(AndroidBridgeError::new(
            "Android не вернул идентификатор установки или FCM token",
        ))),
    }
}

#[cfg(target_os = "android")]
fn optional_java_string(env: &mut JNIEnv<'_>, value: jstring) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: ссылка передана JVM в текущий native callback и живёт до его завершения.
    let value = unsafe { JString::from_raw(value) };
    env.get_string(&value).ok().map(Into::into)
}
