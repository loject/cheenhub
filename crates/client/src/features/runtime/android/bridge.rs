//! Android-реализация реестра Activity/Service bridge.

#[cfg(feature = "android")]
use std::collections::HashMap;
#[cfg(feature = "android")]
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(feature = "android")]
use super::{
    AndroidBridge, AndroidBridgeError, AndroidPermission, ForegroundServiceKind,
    MediaProjectionGrant, PermissionResult,
};
#[cfg(feature = "android")]
use jni::JNIEnv;
#[cfg(feature = "android")]
use jni::objects::{GlobalRef, JObject, JValue};
#[cfg(feature = "android")]
use jni::sys::{jboolean, jint};

#[cfg(feature = "android")]
static ANDROID_BRIDGE: OnceLock<Arc<dyn AndroidBridge>> = OnceLock::new();

#[cfg(feature = "android")]
type PermissionCallback =
    Box<dyn FnOnce(Result<PermissionResult, AndroidBridgeError>) + Send + 'static>;
#[cfg(feature = "android")]
type ProjectionCallback =
    Box<dyn FnOnce(Result<Option<MediaProjectionGrant>, AndroidBridgeError>) + Send + 'static>;

#[cfg(feature = "android")]
static PERMISSION_CALLBACKS: OnceLock<Mutex<HashMap<i32, PermissionCallback>>> = OnceLock::new();
#[cfg(feature = "android")]
static PROJECTION_CALLBACKS: OnceLock<Mutex<HashMap<i32, ProjectionCallback>>> = OnceLock::new();
#[cfg(feature = "android")]
static PROJECTION_GRANTS: OnceLock<Mutex<HashMap<u64, GlobalRef>>> = OnceLock::new();
#[cfg(feature = "android")]
static NEXT_REQUEST_ID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1000);
#[cfg(feature = "android")]
static NEXT_GRANT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[cfg(feature = "android")]
struct JniAndroidBridge;

#[cfg(feature = "android")]
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
        let permission = match permission {
            AndroidPermission::RecordAudio => "android.permission.RECORD_AUDIO",
            AndroidPermission::Camera => "android.permission.CAMERA",
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
}

/// Возвращает установленный Android Activity/Service bridge.
#[cfg(feature = "android")]
pub(crate) fn android_bridge() -> Result<&'static Arc<dyn AndroidBridge>, AndroidBridgeError> {
    Ok(ANDROID_BRIDGE.get_or_init(|| Arc::new(JniAndroidBridge)))
}

#[cfg(feature = "android")]
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

#[cfg(feature = "android")]
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

#[cfg(feature = "android")]
fn next_request_id() -> i32 {
    NEXT_REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(feature = "android")]
fn permission_callbacks() -> &'static Mutex<HashMap<i32, PermissionCallback>> {
    PERMISSION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(feature = "android")]
fn projection_callbacks() -> &'static Mutex<HashMap<i32, ProjectionCallback>> {
    PROJECTION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(feature = "android")]
fn projection_grants() -> &'static Mutex<HashMap<u64, GlobalRef>> {
    PROJECTION_GRANTS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(feature = "android")]
fn lock_error<T>(_error: std::sync::PoisonError<T>) -> AndroidBridgeError {
    AndroidBridgeError::new("Android bridge state повреждён")
}

#[cfg(feature = "android")]
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

#[cfg(feature = "android")]
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
