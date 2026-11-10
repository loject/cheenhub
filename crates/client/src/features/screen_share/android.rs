//! Android MediaProjection через Surface системного `MediaCodec`.

use super::backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCaptureSource,
    ScreenShareCodec, ScreenShareError, ScreenShareSession, ScreenShareStartRequest,
    encoder_config, fit_capture_preset,
};
use crate::features::video_encoding::{
    AndroidSurfaceVideoEncoder, AndroidVideoCaptureSession, AndroidVideoEncodingManager,
    VideoEncodingAcceleratorKind, VideoEncodingManager, VideoFrameEncoder,
    android_video_capture_bridge,
};
use dioxus::logger::tracing::{error, info};
use futures_util::future::LocalBoxFuture;
use std::{cell::Cell, rc::Rc, time::Duration};

/// Android backend MediaProjection.
pub(crate) struct AndroidScreenShareBackend;

impl ScreenShareBackend for AndroidScreenShareBackend {
    fn supports_platform_quality_picker(&self) -> bool {
        true
    }

    fn start(
        &self,
        request: ScreenShareStartRequest,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        Box::pin(async move {
            if matches!(request.source, ScreenShareCaptureSource::Selected { .. }) {
                return Err(ScreenShareError::new(
                    "Выбор физического монитора недоступен в Android backend.",
                ));
            }
            let config = request.config;
            let ScreenShareCaptureSource::PlatformDefault { target } = request.source else {
                unreachable!();
            };
            let (width, height) = screen_dimensions().await?;
            let preset = if let Some(target) = target {
                let settings = config.preset_for_target(&target).ok_or_else(|| {
                    ScreenShareError::new("Выбранное качество демонстрации недоступно.")
                })?;
                fit_capture_preset(width, height, settings)
            } else {
                config.preset_for_capture(width, height)
            };
            info!(
                source_width = width,
                source_height = height,
                width = preset.width,
                height = preset.height,
                fps = preset.max_fps,
                bitrate_bps = preset.bitrate_bps,
                "Android screen capture quality selected"
            );
            let bridge = android_video_capture_bridge().map_err(ScreenShareError::new)?;
            let target = callbacks.on_frame.clone();
            let encoder = AndroidVideoEncodingManager
                .create_encoder(
                    VideoEncodingAcceleratorKind::Native,
                    encoder_config(preset),
                    Rc::new(move |f| {
                        target(EncodedScreenShareFrame {
                            sequence: f.sequence,
                            timestamp_us: f.timestamp_us,
                            duration_us: f.duration_us,
                            codec: ScreenShareCodec::Vp9,
                            key_frame: f.key_frame,
                            width: f.width,
                            height: f.height,
                            bytes: f.bytes,
                        })
                    }),
                )
                .await
                .map_err(|e| ScreenShareError::new(e.to_string()))?;
            let capture = bridge
                .start_screen_share(
                    encoder.input_surface(),
                    preset.width,
                    preset.height,
                    preset.max_fps,
                    callbacks.on_ended,
                )
                .await
                .map_err(ScreenShareError::new)?;
            let session = Rc::new(AndroidScreenShareSession {
                encoder: Rc::new(encoder),
                capture,
                stopped: Rc::new(Cell::new(false)),
            });
            session.start_drain();
            Ok(session as Rc<dyn ScreenShareSession>)
        })
    }
}

async fn screen_dimensions() -> Result<(u32, u32), ScreenShareError> {
    let (sender, receiver) = futures_channel::oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let call = env.call_method(activity, "cheenHubScreenDimensions", "()J", &[]);
        let result = crate::features::runtime::android::guard_jni_result(
            env,
            "cheenHubScreenDimensions",
            call,
        )
        .and_then(|value| value.j())
        .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    let packed = receiver
        .await
        .map_err(|_| ScreenShareError::new("Не удалось определить размер экрана."))?
        .map_err(ScreenShareError::new)? as u64;
    let dimensions = ((packed >> 32) as u32, packed as u32);
    if dimensions.0 == 0 || dimensions.1 == 0 {
        return Err(ScreenShareError::new(
            "Не удалось определить размер экрана.",
        ));
    }
    Ok(dimensions)
}
struct AndroidScreenShareSession {
    encoder: Rc<AndroidSurfaceVideoEncoder>,
    capture: Rc<dyn AndroidVideoCaptureSession>,
    stopped: Rc<Cell<bool>>,
}
impl AndroidScreenShareSession {
    fn start_drain(&self) {
        let encoder = self.encoder.clone();
        let stopped = self.stopped.clone();
        dioxus::prelude::spawn(async move {
            while !stopped.get() {
                if let Err(error) = encoder.drain() {
                    error!(error = %error, "Ошибка Android MediaProjection VP9 encoder");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }
}
impl ScreenShareSession for AndroidScreenShareSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>> {
        self.stopped.set(true);
        let capture = self.capture.clone();
        let encoder = self.encoder.clone();
        Box::pin(async move {
            capture.stop().map_err(ScreenShareError::new)?;
            encoder
                .close()
                .map_err(|e| ScreenShareError::new(e.to_string()))
        })
    }
}
