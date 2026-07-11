//! Android MediaProjection через Surface системного `MediaCodec`.

use super::backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCodec,
    ScreenShareConfig, ScreenShareError, ScreenShareSession,
};
use crate::features::video_encoding::{
    AndroidSurfaceVideoEncoder, AndroidVideoCaptureSession, AndroidVideoEncodingManager,
    VideoEncoderConfig, VideoEncodingAcceleratorKind, VideoEncodingManager, VideoFrameEncoder,
    android_video_capture_bridge,
};
use dioxus::logger::tracing::error;
use futures_util::future::LocalBoxFuture;
use std::{cell::Cell, rc::Rc, time::Duration};

/// Android backend MediaProjection.
pub(crate) struct AndroidScreenShareBackend;

impl ScreenShareBackend for AndroidScreenShareBackend {
    fn start(
        &self,
        config: ScreenShareConfig,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        Box::pin(async move {
            let bridge = android_video_capture_bridge().map_err(ScreenShareError::new)?;
            let target = callbacks.on_frame.clone();
            let encoder = AndroidVideoEncodingManager
                .create_encoder(
                    VideoEncodingAcceleratorKind::Native,
                    VideoEncoderConfig::vp9(
                        config.fallback_width,
                        config.fallback_height,
                        config.frame_rate,
                        config.bitrate_bps,
                    ),
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
                    config.fallback_width,
                    config.fallback_height,
                    config.frame_rate,
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
