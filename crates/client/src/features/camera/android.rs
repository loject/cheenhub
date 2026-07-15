//! Android-захват камеры через Surface системного `MediaCodec`.

use super::backend::{
    CameraBackend, CameraCallbacks, CameraCodec, CameraConfig, CameraError, CameraSession,
    EncodedCameraFrame,
};
use crate::features::video_encoding::{
    AndroidSurfaceVideoEncoder, AndroidVideoCaptureSession, AndroidVideoEncodingManager,
    VideoEncoderConfig, VideoEncodingAcceleratorKind, VideoEncodingManager, VideoFrameEncoder,
    android_video_capture_bridge,
};
use dioxus::logger::tracing::error;
use futures_util::future::LocalBoxFuture;
use std::{cell::Cell, rc::Rc, time::Duration};

/// Android backend Camera2.
pub(crate) struct AndroidCameraBackend;

impl CameraBackend for AndroidCameraBackend {
    fn start(
        &self,
        config: CameraConfig,
        callbacks: CameraCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn CameraSession>, CameraError>> {
        Box::pin(async move {
            let preset = config.preset_spec();
            let bridge = android_video_capture_bridge().map_err(CameraError::new)?;
            let target = callbacks.on_frame.clone();
            let encoder = AndroidVideoEncodingManager
                .create_encoder(
                    VideoEncodingAcceleratorKind::Native,
                    VideoEncoderConfig::vp9(
                        preset.width,
                        preset.height,
                        preset.max_fps,
                        preset.bitrate_bps,
                    ),
                    Rc::new(move |f| {
                        target(EncodedCameraFrame {
                            sequence: f.sequence,
                            timestamp_us: f.timestamp_us,
                            duration_us: f.duration_us,
                            codec: CameraCodec::Vp9,
                            key_frame: f.key_frame,
                            width: f.width,
                            height: f.height,
                            bytes: f.bytes,
                        })
                    }),
                )
                .await
                .map_err(|e| CameraError::new(e.to_string()))?;
            let capture = bridge
                .start_camera(
                    encoder.input_surface(),
                    preset.width,
                    preset.height,
                    preset.max_fps,
                    callbacks.on_ended,
                )
                .await
                .map_err(CameraError::new)?;
            let session = Rc::new(AndroidCameraSession {
                encoder: Rc::new(encoder),
                capture,
                stopped: Rc::new(Cell::new(false)),
            });
            session.start_drain();
            Ok(session as Rc<dyn CameraSession>)
        })
    }
}

struct AndroidCameraSession {
    encoder: Rc<AndroidSurfaceVideoEncoder>,
    capture: Rc<dyn AndroidVideoCaptureSession>,
    stopped: Rc<Cell<bool>>,
}
impl AndroidCameraSession {
    fn start_drain(&self) {
        let encoder = self.encoder.clone();
        let stopped = self.stopped.clone();
        dioxus::prelude::spawn(async move {
            while !stopped.get() {
                if let Err(error) = encoder.drain() {
                    error!(error = %error, "Ошибка Android camera VP9 encoder");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }
}
impl CameraSession for AndroidCameraSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), CameraError>> {
        self.stopped.set(true);
        let capture = self.capture.clone();
        let encoder = self.encoder.clone();
        Box::pin(async move {
            capture.stop().map_err(CameraError::new)?;
            encoder.close().map_err(|e| CameraError::new(e.to_string()))
        })
    }
}
