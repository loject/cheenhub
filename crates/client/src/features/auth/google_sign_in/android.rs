//! Android-реализация входа через Google Credential Manager.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use dioxus::logger::tracing::{debug, warn};
use futures_channel::oneshot;
use jni::JNIEnv;
use jni::objects::{JObject, JString, JValue};
use jni::sys::{jboolean, jint, jstring};

use crate::features::auth::google_sign_in::GoogleSignInError;

type GoogleSignInCallback =
    Box<dyn FnOnce(Result<Option<String>, GoogleSignInError>) + Send + 'static>;

static CALLBACKS: OnceLock<Mutex<HashMap<i32, GoogleSignInCallback>>> = OnceLock::new();
static NEXT_REQUEST_ID: AtomicI32 = AtomicI32::new(20_000);

pub(in crate::features::auth::google_sign_in) const fn is_supported() -> bool {
    true
}

pub(in crate::features::auth::google_sign_in) async fn request_google_id_token(
    server_client_id: String,
    nonce: String,
) -> Result<Option<String>, GoogleSignInError> {
    if server_client_id.trim().is_empty() {
        return Err(GoogleSignInError::new(
            "Backend не вернул Google Web Client ID",
        ));
    }
    if nonce.trim().is_empty() {
        return Err(GoogleSignInError::new(
            "Backend не вернул nonce для входа через Google",
        ));
    }

    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = oneshot::channel();
    callbacks().lock().map_err(lock_error)?.insert(
        request_id,
        Box::new(move |result| {
            let _ = sender.send(result);
        }),
    );

    debug!(request_id, "requesting Google ID token from Android");
    wry::prelude::dispatch(move |env, activity, _| {
        let result = env
            .new_string(server_client_id)
            .and_then(|server_client_id| {
                let nonce = env.new_string(nonce)?;
                env.call_method(
                    activity,
                    "requestCheenHubGoogleIdToken",
                    "(ILjava/lang/String;Ljava/lang/String;)V",
                    &[
                        JValue::Int(request_id),
                        JValue::Object(&server_client_id),
                        JValue::Object(&nonce),
                    ],
                )
                .map(|_| ())
            });
        if let Err(error) = result {
            warn!(request_id, %error, "failed to dispatch Android Google sign-in");
            finish(
                request_id,
                Err(GoogleSignInError::new(format!(
                    "Не удалось открыть системный вход через Google: {error}"
                ))),
            );
        }
    });

    receiver
        .await
        .map_err(|_| GoogleSignInError::new("Android закрыл callback входа через Google"))?
}

fn callbacks() -> &'static Mutex<HashMap<i32, GoogleSignInCallback>> {
    CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> GoogleSignInError {
    GoogleSignInError::new("Состояние Android Google sign-in повреждено")
}

fn finish(request_id: i32, result: Result<Option<String>, GoogleSignInError>) {
    let callback = callbacks()
        .lock()
        .ok()
        .and_then(|mut callbacks| callbacks.remove(&request_id));
    if let Some(callback) = callback {
        callback(result);
    }
}

/// Завершает асинхронный Android Credential Manager request.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubGoogleIdTokenResult(
    mut env: JNIEnv<'_>,
    _activity: JObject<'_>,
    request_id: jint,
    id_token: jstring,
    error_code: jstring,
    cancelled: jboolean,
) {
    if cancelled != 0 {
        debug!(request_id, "Android Google sign-in was cancelled");
        finish(request_id, Ok(None));
        return;
    }
    if let Some(error_code) = optional_java_string(&mut env, error_code) {
        warn!(request_id, %error_code, "Android Google sign-in failed");
        finish(
            request_id,
            Err(GoogleSignInError::new(format!(
                "Системный вход через Google недоступен: {error_code}"
            ))),
        );
        return;
    }
    let Some(id_token) =
        optional_java_string(&mut env, id_token).filter(|value| !value.trim().is_empty())
    else {
        finish(
            request_id,
            Err(GoogleSignInError::new("Android не вернул Google ID token")),
        );
        return;
    };

    debug!(request_id, "Android Google sign-in completed");
    finish(request_id, Ok(Some(id_token)));
}

fn optional_java_string(env: &mut JNIEnv<'_>, value: jstring) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: ссылка передана JVM в текущий native callback и живёт до его завершения.
    let value = unsafe { JString::from_raw(value) };
    env.get_string(&value).ok().map(Into::into)
}
