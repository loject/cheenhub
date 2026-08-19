//! Нативный Android Photo Picker для изображения сообщения.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use dioxus::prelude::*;
use futures_channel::oneshot;
use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jbyteArray, jint, jstring};

use super::super::backend::{ImagePickerOutcome, PickedImage, oversized_image_message};

#[manganis::ffi("android/image-picker")]
unsafe extern "Kotlin" {}

type PickerSender = oneshot::Sender<Result<Option<PickedImage>, String>>;

static PICKER_REQUESTS: OnceLock<Mutex<HashMap<i32, PickerSender>>> = OnceLock::new();
static NEXT_REQUEST_ID: AtomicI32 = AtomicI32::new(30_000);

/// Показывает кнопку системного Android Photo Picker для одного изображения.
#[component]
pub(crate) fn ImagePickerButton(
    disabled: bool,
    busy: bool,
    max_bytes: usize,
    on_outcome: EventHandler<ImagePickerOutcome>,
    on_active_change: EventHandler<bool>,
) -> Element {
    let mut is_picking = use_signal(|| false);
    let unavailable = disabled || busy || is_picking();

    rsx! {
        button {
            r#type: "button",
            disabled: unavailable,
            class: "flex size-10 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-300 transition-[background-color,border-color,color,transform,opacity] duration-150 ease-out hover:-translate-y-px hover:border-white/15 hover:bg-zinc-800 hover:text-zinc-100 active:scale-[0.96] disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:translate-y-0 disabled:active:scale-100",
            title: "Прикрепить изображение",
            "aria-label": "Прикрепить изображение",
            onclick: move |_| {
                if disabled || busy || is_picking() {
                    return;
                }
                is_picking.set(true);
                on_active_change.call(true);
                spawn(async move {
                    match pick_image().await {
                        Ok(Some(image)) if image.bytes.len() > max_bytes => {
                            on_outcome.call(ImagePickerOutcome::Failed(
                                oversized_image_message(max_bytes),
                            ));
                        }
                        Ok(Some(image)) => {
                            on_outcome.call(ImagePickerOutcome::Selected(image));
                        }
                        Ok(None) => {
                            debug!("Android image picker was cancelled");
                        }
                        Err(error) => {
                            warn!(%error, "Android image picker failed");
                            on_outcome.call(ImagePickerOutcome::Failed(error));
                        }
                    }
                    is_picking.set(false);
                    on_active_change.call(false);
                });
            },
            if busy || is_picking() {
                span { class: "size-4 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300", "aria-hidden": "true" }
            } else {
                svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "m18.375 12.739-7.693 7.693a4.5 4.5 0 0 1-6.364-6.364l10.94-10.94a3 3 0 1 1 4.243 4.243L8.552 18.32a1.5 1.5 0 1 1-2.121-2.121l9.879-9.879" }
                }
            }
        }
    }
}

async fn pick_image() -> Result<Option<PickedImage>, String> {
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = oneshot::channel();
    picker_requests()
        .lock()
        .map_err(|_| "Состояние Android Photo Picker повреждено.".to_owned())?
        .insert(request_id, sender);

    debug!(request_id, "opening Android image picker");
    wry::prelude::dispatch(move |env, activity, _| {
        let result = (|| -> Result<(), String> {
            let class = load_app_class(
                env,
                activity,
                "ru.cheenhub.imagepicker.CheenHubImagePickerPlugin",
            )?;
            let plugin = env
                .new_object(
                    class,
                    "(Landroid/app/Activity;)V",
                    &[JValue::Object(activity)],
                )
                .map_err(|error| {
                    clear_jni_exception(env, "Android image picker init failed", error)
                })?;
            let started = env
                .call_method(&plugin, "pickImage", "(I)Z", &[JValue::Int(request_id)])
                .and_then(|value| value.z())
                .map_err(|error| {
                    clear_jni_exception(env, "Android image picker launch failed", error)
                })?;
            if !started {
                return Err("Android не смог открыть системный выбор изображения.".to_owned());
            }
            Ok(())
        })();
        if let Err(error) = result {
            finish_request(request_id, Err(error));
        }
    });

    receiver
        .await
        .map_err(|_| "Android закрыл callback выбора изображения.".to_owned())?
}

fn load_app_class<'local>(
    env: &mut JNIEnv<'local>,
    activity: &JObject<'_>,
    class_name: &str,
) -> Result<JClass<'local>, String> {
    let class_loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .and_then(|value| value.l())
        .map_err(|error| clear_jni_exception(env, "Android app ClassLoader unavailable", error))?;
    let class_name = env
        .new_string(class_name)
        .map_err(|error| clear_jni_exception(env, "Android picker class name invalid", error))?;
    let class_name = JObject::from(class_name);
    let class = env
        .call_method(
            class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name)],
        )
        .and_then(|value| value.l())
        .map_err(|error| {
            clear_jni_exception(env, "Android image picker class unavailable", error)
        })?;
    Ok(JClass::from(class))
}

fn clear_jni_exception(env: &mut JNIEnv<'_>, context: &str, error: jni::errors::Error) -> String {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    format!("{context}: {error}")
}

fn picker_requests() -> &'static Mutex<HashMap<i32, PickerSender>> {
    PICKER_REQUESTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn finish_request(request_id: i32, result: Result<Option<PickedImage>, String>) {
    let sender = picker_requests()
        .lock()
        .ok()
        .and_then(|mut requests| requests.remove(&request_id));
    if let Some(sender) = sender {
        let _ = sender.send(result);
    }
}

/// Завершает асинхронный Android Photo Picker request.
#[unsafe(no_mangle)]
pub extern "system" fn Java_ru_cheenhub_imagepicker_CheenHubImagePickerActivity_nativeOnCheenHubImagePickerResult(
    mut env: JNIEnv<'_>,
    _activity: JObject<'_>,
    request_id: jint,
    file_name: jstring,
    bytes: jbyteArray,
    error_code: jstring,
    cancelled: jboolean,
) {
    if cancelled != 0 {
        finish_request(request_id, Ok(None));
        return;
    }
    if let Some(error_code) = optional_java_string(&mut env, error_code) {
        finish_request(request_id, Err(error_message(&error_code)));
        return;
    }
    if bytes.is_null() {
        finish_request(
            request_id,
            Err("Android не вернул выбранное изображение.".to_owned()),
        );
        return;
    }

    // SAFETY: массив передан JVM в текущий native callback и живёт до его завершения.
    let bytes = unsafe { JByteArray::from_raw(bytes) };
    let bytes = match env.convert_byte_array(&bytes) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        Ok(_) => {
            finish_request(request_id, Err("Выбранное изображение пустое.".to_owned()));
            return;
        }
        Err(error) => {
            finish_request(
                request_id,
                Err(format!(
                    "Не удалось прочитать выбранное Android-изображение: {error}"
                )),
            );
            return;
        }
    };
    let file_name =
        optional_java_string(&mut env, file_name).filter(|file_name| !file_name.trim().is_empty());
    info!(
        request_id,
        byte_size = bytes.len(),
        "Android image selected"
    );
    finish_request(request_id, Ok(Some(PickedImage { file_name, bytes })));
}

fn error_message(error_code: &str) -> String {
    match error_code {
        "image_too_large" => "Изображение слишком большое. Максимум — 10 МБ.".to_owned(),
        "unsupported_image" => "Выберите изображение PNG, JPEG, GIF или WebP.".to_owned(),
        "empty_image" => "Выбранное изображение пустое.".to_owned(),
        "picker_unavailable" => {
            "Системный выбор изображений недоступен на этом устройстве.".to_owned()
        }
        _ => "Не удалось прочитать выбранное изображение.".to_owned(),
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
