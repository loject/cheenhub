//! Android-backend микрофона с foreground service и захватом через `cpal`.

use std::rc::Rc;

use dioxus::prelude::{info, warn};
use futures_util::FutureExt;

use super::cpal_capture::CpalMicrophoneBackend;
use crate::features::runtime::android::{
    AndroidPermission, ForegroundServiceKind, PermissionResult, android_bridge,
};

use super::super::backend::{
    MicrophoneBackend, MicrophoneCallbacks, MicrophoneConfig, MicrophoneError, MicrophoneSession,
};

/// Android-backend микрофона, владеющий voice foreground service.
#[derive(Debug, Default)]
pub(super) struct AndroidMicrophoneBackend;

impl MicrophoneBackend for AndroidMicrophoneBackend {
    fn start(
        &self,
        config: MicrophoneConfig,
        callbacks: MicrophoneCallbacks,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<Rc<dyn MicrophoneSession>, MicrophoneError>,
    > {
        async move {
            let bridge = android_bridge().map_err(android_error)?;
            let (sender, receiver) = futures_channel::oneshot::channel();
            bridge
                .request_permission(
                    AndroidPermission::RecordAudio,
                    Box::new(move |result| {
                        let _ = sender.send(result);
                    }),
                )
                .map_err(android_error)?;
            match receiver
                .await
                .map_err(|_| MicrophoneError::new("Android не вернул результат запроса микрофона."))?
                .map_err(android_error)?
            {
                PermissionResult::Granted => {}
                PermissionResult::Denied | PermissionResult::DeniedPermanently => {
                    return Err(MicrophoneError::permission_denied(
                        "Доступ к микрофону запрещён в Android.",
                    ));
                }
            }
            bridge
                .start_foreground_service(ForegroundServiceKind::Voice)
                .map_err(android_error)?;

            let backend = CpalMicrophoneBackend;
            match backend.start(config, callbacks).await {
                Ok(session) => {
                    info!("android microphone foreground session started");
                    Ok(Rc::new(AndroidMicrophoneSession { inner: session })
                        as Rc<dyn MicrophoneSession>)
                }
                Err(error) => {
                    if let Err(stop_error) =
                        bridge.stop_foreground_service(ForegroundServiceKind::Voice)
                    {
                        warn!(
                            error = %stop_error,
                            "failed to stop Android voice foreground service after microphone startup failure"
                        );
                    }
                    Err(error)
                }
            }
        }
        .boxed_local()
    }
}

struct AndroidMicrophoneSession {
    inner: Rc<dyn MicrophoneSession>,
}

impl MicrophoneSession for AndroidMicrophoneSession {
    fn stop(&self) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        let inner = self.inner.clone();
        async move {
            let capture_result = inner.stop().await;
            let service_result = android_bridge().map_err(android_error).and_then(|bridge| {
                bridge
                    .stop_foreground_service(ForegroundServiceKind::Voice)
                    .map_err(android_error)
            });

            capture_result?;
            service_result?;
            info!("android microphone foreground session stopped");
            Ok(())
        }
        .boxed_local()
    }

    fn set_bitrate_bps(
        &self,
        bitrate_bps: u32,
    ) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        self.inner.set_bitrate_bps(bitrate_bps)
    }
}

fn android_error(error: impl std::fmt::Display) -> MicrophoneError {
    MicrophoneError::new(format!("Android audio bridge вернул ошибку: {error}"))
}
