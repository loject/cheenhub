//! Android bridge захвата камеры и экрана.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use futures_util::future::LocalBoxFuture;
use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject, JValue};

use crate::features::runtime::android::{
    AndroidPermission, ForegroundServiceKind, PermissionResult, android_bridge,
    take_media_projection_grant,
};

use super::encoder::AndroidEncoderSurface;

/// Активный Android-источник кадров, подключенный к encoder Surface.
pub(crate) trait AndroidVideoCaptureSession {
    /// Останавливает источник и освобождает Camera2/MediaProjection ресурсы.
    fn stop(&self) -> Result<(), String>;
}

/// Адаптер Activity для Camera2 и MediaProjection.
pub(crate) trait AndroidVideoCaptureBridge {
    /// Направляет камеру в encoder Surface после проверки permission.
    fn start_camera(
        &self,
        surface: AndroidEncoderSurface,
        width: u32,
        height: u32,
        frame_rate: u32,
        on_ended: Rc<dyn Fn()>,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn AndroidVideoCaptureSession>, String>>;
    /// Получает MediaProjection consent и создаёт VirtualDisplay для Surface.
    fn start_screen_share(
        &self,
        surface: AndroidEncoderSurface,
        width: u32,
        height: u32,
        frame_rate: u32,
        on_ended: Rc<dyn Fn()>,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn AndroidVideoCaptureSession>, String>>;
}

thread_local! {
    static CAPTURE_ENDED_CALLBACKS: RefCell<HashMap<i32, Rc<dyn Fn()>>> = RefCell::new(HashMap::new());
}
static NEXT_CAPTURE_ID: AtomicI32 = AtomicI32::new(1);
static ENDED_EVENTS: OnceLock<Mutex<Vec<i32>>> = OnceLock::new();

/// Устанавливает Activity-адаптер до создания media provider'ов.
/// Возвращает установленный Activity-адаптер.
pub(crate) fn android_video_capture_bridge() -> Result<Rc<dyn AndroidVideoCaptureBridge>, String> {
    ensure_ended_event_relay();
    Ok(Rc::new(JniAndroidVideoCaptureBridge))
}

struct JniAndroidVideoCaptureBridge;
struct JniAndroidVideoCaptureSession {
    id: i32,
    service: ForegroundServiceKind,
    stopped: Cell<bool>,
}

impl AndroidVideoCaptureSession for JniAndroidVideoCaptureSession {
    fn stop(&self) -> Result<(), String> {
        if self.stopped.replace(true) {
            return Ok(());
        }
        CAPTURE_ENDED_CALLBACKS.with(|callbacks| callbacks.borrow_mut().remove(&self.id));
        let id = self.id;
        wry::prelude::dispatch(move |env, activity, _| {
            let _ = env.call_method(activity, "stopCheenHubCapture", "(I)V", &[JValue::Int(id)]);
        });
        android_bridge()
            .map_err(|e| e.to_string())?
            .stop_foreground_service(self.service)
            .map_err(|e| e.to_string())
    }
}

impl AndroidVideoCaptureBridge for JniAndroidVideoCaptureBridge {
    fn start_camera(
        &self,
        surface: AndroidEncoderSurface,
        width: u32,
        height: u32,
        frame_rate: u32,
        on_ended: Rc<dyn Fn()>,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn AndroidVideoCaptureSession>, String>> {
        Box::pin(async move {
            request_camera_permission().await?;
            let platform = android_bridge().map_err(|e| e.to_string())?;
            platform
                .start_foreground_service(ForegroundServiceKind::Camera)
                .map_err(|e| e.to_string())?;
            let id = next_capture_id();
            let ended = Rc::new(move || {
                if let Ok(platform) = android_bridge() {
                    let _ = platform.stop_foreground_service(ForegroundServiceKind::Camera);
                }
                on_ended();
            });
            CAPTURE_ENDED_CALLBACKS.with(|callbacks| callbacks.borrow_mut().insert(id, ended));
            if let Err(error) = dispatch_camera_start(id, surface, width, height, frame_rate).await
            {
                CAPTURE_ENDED_CALLBACKS.with(|callbacks| callbacks.borrow_mut().remove(&id));
                let _ = platform.stop_foreground_service(ForegroundServiceKind::Camera);
                return Err(error);
            }
            Ok(Rc::new(JniAndroidVideoCaptureSession {
                id,
                service: ForegroundServiceKind::Camera,
                stopped: Cell::new(false),
            }) as Rc<dyn AndroidVideoCaptureSession>)
        })
    }

    fn start_screen_share(
        &self,
        surface: AndroidEncoderSurface,
        width: u32,
        height: u32,
        _frame_rate: u32,
        on_ended: Rc<dyn Fn()>,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn AndroidVideoCaptureSession>, String>> {
        Box::pin(async move {
            let platform = android_bridge().map_err(|e| e.to_string())?;
            let (sender, receiver) = futures_channel::oneshot::channel();
            platform
                .request_media_projection(Box::new(move |result| {
                    let _ = sender.send(result);
                }))
                .map_err(|e| e.to_string())?;
            let grant = receiver
                .await
                .map_err(|_| "MediaProjection consent callback потерян".to_owned())?
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Пользователь отменил демонстрацию экрана".to_owned())?;
            let intent = take_media_projection_grant(grant).map_err(|e| e.to_string())?;
            platform
                .start_foreground_service(ForegroundServiceKind::MediaProjection)
                .map_err(|e| e.to_string())?;
            let id = next_capture_id();
            let ended = Rc::new(move || {
                if let Ok(platform) = android_bridge() {
                    let _ =
                        platform.stop_foreground_service(ForegroundServiceKind::MediaProjection);
                }
                on_ended();
            });
            CAPTURE_ENDED_CALLBACKS.with(|callbacks| callbacks.borrow_mut().insert(id, ended));
            if let Err(error) = dispatch_screen_start(id, intent, surface, width, height).await {
                CAPTURE_ENDED_CALLBACKS.with(|callbacks| callbacks.borrow_mut().remove(&id));
                let _ = platform.stop_foreground_service(ForegroundServiceKind::MediaProjection);
                return Err(error);
            }
            Ok(Rc::new(JniAndroidVideoCaptureSession {
                id,
                service: ForegroundServiceKind::MediaProjection,
                stopped: Cell::new(false),
            }) as Rc<dyn AndroidVideoCaptureSession>)
        })
    }
}

async fn request_camera_permission() -> Result<(), String> {
    let (sender, receiver) = futures_channel::oneshot::channel();
    android_bridge()
        .map_err(|e| e.to_string())?
        .request_permission(
            AndroidPermission::Camera,
            Box::new(move |result| {
                let _ = sender.send(result);
            }),
        )
        .map_err(|e| e.to_string())?;
    match receiver
        .await
        .map_err(|_| "Android camera permission callback потерян".to_owned())?
        .map_err(|e| e.to_string())?
    {
        PermissionResult::Granted => Ok(()),
        PermissionResult::Denied => Err("Доступ к камере отклонён".into()),
        PermissionResult::DeniedPermanently => {
            Err("Доступ к камере запрещён в настройках Android".into())
        }
    }
}

async fn dispatch_camera_start(
    id: i32,
    surface: AndroidEncoderSurface,
    width: u32,
    height: u32,
    fps: u32,
) -> Result<(), String> {
    let (sender, receiver) = futures_channel::oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let result = env
            .call_method(
                activity,
                "startCheenHubCamera",
                "(ILandroid/view/Surface;III)V",
                &[
                    JValue::Int(id),
                    JValue::Object(surface.as_obj()),
                    JValue::Int(width as i32),
                    JValue::Int(height as i32),
                    JValue::Int(fps as i32),
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string());
        let _ = sender.send(result);
    });
    receiver
        .await
        .map_err(|_| "Android Camera2 start callback потерян".to_owned())?
}

async fn dispatch_screen_start(
    id: i32,
    intent: GlobalRef,
    surface: AndroidEncoderSurface,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let (sender, receiver) = futures_channel::oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let result = env
            .call_method(
                activity,
                "startCheenHubScreenShare",
                "(ILandroid/content/Intent;Landroid/view/Surface;II)V",
                &[
                    JValue::Int(id),
                    JValue::Object(intent.as_obj()),
                    JValue::Object(surface.as_obj()),
                    JValue::Int(width as i32),
                    JValue::Int(height as i32),
                ],
            )
            .map(|_| ())
            .map_err(|e| e.to_string());
        let _ = sender.send(result);
    });
    receiver
        .await
        .map_err(|_| "Android MediaProjection start callback потерян".to_owned())?
}

fn next_capture_id() -> i32 {
    NEXT_CAPTURE_ID.fetch_add(1, Ordering::Relaxed)
}

fn ensure_ended_event_relay() {
    thread_local! { static STARTED: Cell<bool> = const { Cell::new(false) }; }
    STARTED.with(|started| {
        if started.replace(true) {
            return;
        }
        dioxus::prelude::spawn(async {
            loop {
                let ids = ENDED_EVENTS
                    .get_or_init(|| Mutex::new(Vec::new()))
                    .lock()
                    .map(|mut events| std::mem::take(&mut *events))
                    .unwrap_or_default();
                for id in ids {
                    if let Some(callback) =
                        CAPTURE_ENDED_CALLBACKS.with(|callbacks| callbacks.borrow_mut().remove(&id))
                    {
                        callback();
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    });
}

/// JNI callback системного завершения Camera2 или MediaProjection capture.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubCaptureEnded(
    _env: JNIEnv<'_>,
    _activity: JObject<'_>,
    capture_id: jni::sys::jint,
) {
    if let Ok(mut events) = ENDED_EVENTS.get_or_init(|| Mutex::new(Vec::new())).lock() {
        events.push(capture_id);
    }
}
