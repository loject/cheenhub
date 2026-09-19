//! Реестр и JNI callbacks Android bridge.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject, JString};
use jni::sys::{jboolean, jint, jstring};

use super::super::{
    AndroidBridgeError, AndroidPushInstallation, MediaProjectionGrant, PermissionResult,
};

pub(super) type PermissionCallback =
    Box<dyn FnOnce(Result<PermissionResult, AndroidBridgeError>) + Send + 'static>;
pub(super) type ProjectionCallback =
    Box<dyn FnOnce(Result<Option<MediaProjectionGrant>, AndroidBridgeError>) + Send + 'static>;
pub(super) type PushInstallationCallback =
    Box<dyn FnOnce(Result<AndroidPushInstallation, AndroidBridgeError>) + Send + 'static>;

static PERMISSION_CALLBACKS: OnceLock<Mutex<HashMap<i32, PermissionCallback>>> = OnceLock::new();
static PROJECTION_CALLBACKS: OnceLock<Mutex<HashMap<i32, ProjectionCallback>>> = OnceLock::new();
static PUSH_INSTALLATION_CALLBACKS: OnceLock<Mutex<HashMap<i32, PushInstallationCallback>>> =
    OnceLock::new();
static PROJECTION_GRANTS: OnceLock<Mutex<HashMap<u64, GlobalRef>>> = OnceLock::new();
static NEXT_REQUEST_ID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1000);
static NEXT_GRANT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(super) fn next_request_id() -> i32 {
    NEXT_REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub(super) fn permission_callbacks() -> &'static Mutex<HashMap<i32, PermissionCallback>> {
    PERMISSION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn projection_callbacks() -> &'static Mutex<HashMap<i32, ProjectionCallback>> {
    PROJECTION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn push_installation_callbacks() -> &'static Mutex<HashMap<i32, PushInstallationCallback>>
{
    PUSH_INSTALLATION_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn finish_permission_request(
    request_id: i32,
    result: Result<PermissionResult, AndroidBridgeError>,
) {
    let callback = permission_callbacks()
        .lock()
        .ok()
        .and_then(|mut callbacks| callbacks.remove(&request_id));
    if let Some(callback) = callback {
        callback(result);
    }
}

pub(super) fn finish_projection_request(
    request_id: i32,
    result: Result<Option<MediaProjectionGrant>, AndroidBridgeError>,
) {
    let callback = projection_callbacks()
        .lock()
        .ok()
        .and_then(|mut callbacks| callbacks.remove(&request_id));
    if let Some(callback) = callback {
        callback(result);
    }
}

pub(super) fn finish_push_installation_request(
    request_id: i32,
    result: Result<AndroidPushInstallation, AndroidBridgeError>,
) {
    let callback = push_installation_callbacks()
        .lock()
        .ok()
        .and_then(|mut callbacks| callbacks.remove(&request_id));
    if let Some(callback) = callback {
        callback(result);
    }
}

fn projection_grants() -> &'static Mutex<HashMap<u64, GlobalRef>> {
    PROJECTION_GRANTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn lock_error<T>(_error: std::sync::PoisonError<T>) -> AndroidBridgeError {
    AndroidBridgeError::new("Android bridge state повреждён")
}

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

fn optional_java_string(env: &mut JNIEnv<'_>, value: jstring) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: ссылка передана JVM в текущий native callback и живёт до его завершения.
    let value = unsafe { JString::from_raw(value) };
    env.get_string(&value).ok().map(Into::into)
}
