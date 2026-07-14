//! Browser-backend демонстрации экрана.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::warn;
use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;
use wasm_bindgen_futures::spawn_local;

use crate::features::video_encoding::{
    BrowserVideoEncoder, BrowserVideoEncoderHandle, BrowserVideoEncodingManager,
    BrowserVideoFrameReader, BrowserVideoFrameReaderHandle, EncodedVideoFrame, VideoCodec,
    VideoEncoderConfig, VideoEncodingAcceleratorKind, VideoEncodingError, VideoEncodingManager,
    VideoFrameEncoder,
};

use super::backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCodec,
    ScreenShareConfig, ScreenShareError, ScreenShareErrorKind, ScreenShareSession,
};
use super::browser_capture::{
    first_video_track, log_selected_video_track, request_screen_stream, video_track_settings,
};

const KEY_FRAME_INTERVAL_SECONDS: u32 = 2;
const UNSUPPORTED_SCREEN_SHARE_MESSAGE: &str = concat!(
    "Этот браузер не поддерживает демонстрацию экрана в CheenHub. ",
    "Воспользуйтесь браузером на базе Chromium или нативным клиентом."
);

/// Реализация браузерной демонстрации экрана на основе `getDisplayMedia` и WebCodecs.
pub(crate) struct BrowserScreenShareBackend;

impl ScreenShareBackend for BrowserScreenShareBackend {
    fn start(
        &self,
        config: ScreenShareConfig,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        async move { start_browser_session(config, callbacks).await }.boxed_local()
    }
}

struct BrowserScreenShareSession {
    encoder: BrowserVideoEncoder,
    track: web_sys::MediaStreamTrack,
    frame_reader: BrowserVideoFrameReaderHandle,
    closed: Rc<Cell<bool>>,
}

impl ScreenShareSession for BrowserScreenShareSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>> {
        let encoder = self.encoder.handle();
        let track = self.track.clone();
        let frame_reader = self.frame_reader.clone();
        let closed = self.closed.clone();
        async move {
            if closed.replace(true) {
                return Ok(());
            }
            frame_reader.stop();
            track.stop();
            encoder.close().map_err(screen_share_video_encoding_error)?;
            Ok(())
        }
        .boxed_local()
    }
}

async fn start_browser_session(
    config: ScreenShareConfig,
    callbacks: ScreenShareCallbacks,
) -> Result<Rc<dyn ScreenShareSession>, ScreenShareError> {
    if config.codec != ScreenShareCodec::Vp9 {
        return Err(ScreenShareError::new(
            "Поддерживается только VP9 демонстрация экрана.",
        ));
    }
    ensure_browser_screen_share_support()?;

    let stream = request_screen_stream().await?;
    let track = first_video_track(&stream)?;
    log_selected_video_track(&track);
    let settings = video_track_settings(&track);
    let width = settings.width.unwrap_or(config.fallback_width).max(1);
    let height = settings.height.unwrap_or(config.fallback_height).max(1);
    let frame_rate = settings.frame_rate.unwrap_or(config.frame_rate).max(1);
    let encoder_config = VideoEncoderConfig::vp9(width, height, frame_rate, config.bitrate_bps);
    ensure_video_encoder_available(&track, encoder_config.clone()).await?;

    let output_on_frame = callbacks.on_frame.clone();
    let on_video_frame = Rc::new(move |frame: EncodedVideoFrame| {
        output_on_frame(screen_share_frame_from_video(frame));
    });
    let manager = BrowserVideoEncodingManager;
    let encoder = match manager
        .create_encoder(
            VideoEncodingAcceleratorKind::WebCodecs,
            encoder_config,
            on_video_frame,
        )
        .await
    {
        Ok(encoder) => encoder,
        Err(error) => {
            track.stop();
            return Err(screen_share_video_encoding_error(error));
        }
    };
    let frame_reader = match BrowserVideoFrameReader::from_stream(&track, &stream).await {
        Ok(reader) => reader,
        Err(error) => {
            let _ = encoder.close();
            track.stop();
            return Err(screen_share_video_encoding_error(error));
        }
    };
    let frame_reader_handle = frame_reader.handle();

    let closed = Rc::new(Cell::new(false));
    let key_frame_interval_frames = frame_rate.saturating_mul(KEY_FRAME_INTERVAL_SECONDS).max(1);
    spawn_video_reader(
        frame_reader,
        encoder.handle(),
        closed.clone(),
        callbacks,
        key_frame_interval_frames,
    );

    Ok(Rc::new(BrowserScreenShareSession {
        encoder,
        track,
        frame_reader: frame_reader_handle,
        closed,
    }))
}

async fn ensure_video_encoder_available(
    track: &web_sys::MediaStreamTrack,
    encoder_config: VideoEncoderConfig,
) -> Result<(), ScreenShareError> {
    let manager = BrowserVideoEncodingManager;
    let available = match manager.available_accelerators(encoder_config).await {
        Ok(available) => available,
        Err(error) => {
            track.stop();
            return Err(screen_share_video_encoding_error(error));
        }
    };
    if !available
        .iter()
        .any(|encoder| encoder.kind == VideoEncodingAcceleratorKind::WebCodecs)
    {
        track.stop();
        return Err(ScreenShareError::new(
            "Браузер не поддерживает кодирование демонстрации экрана в VP9.",
        ));
    }

    Ok(())
}

fn ensure_browser_screen_share_support() -> Result<(), ScreenShareError> {
    let manager = BrowserVideoEncodingManager;
    if !manager.browser_capture_pipeline_available() {
        return Err(ScreenShareError::with_kind(
            UNSUPPORTED_SCREEN_SHARE_MESSAGE,
            ScreenShareErrorKind::UnsupportedBrowser,
        ));
    }

    Ok(())
}

fn spawn_video_reader(
    reader: BrowserVideoFrameReader,
    encoder: BrowserVideoEncoderHandle,
    closed: Rc<Cell<bool>>,
    callbacks: ScreenShareCallbacks,
    key_frame_interval_frames: u32,
) {
    spawn_local(async move {
        let frame_sequence = Rc::new(Cell::new(0_u64));
        while !closed.get() {
            let frame = match reader.read().await {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(error) => {
                    warn!(%error, "failed to read screen sharing frame");
                    break;
                }
            };
            let sequence = frame_sequence.get();
            frame_sequence.set(sequence.saturating_add(1));
            let key_frame = sequence.is_multiple_of(u64::from(key_frame_interval_frames));
            if let Err(error) = encoder.encode(&frame, key_frame) {
                warn!(%error, "failed to encode screen sharing frame");
                frame.close();
                break;
            }
            frame.close();
        }

        finish_browser_capture(&encoder, &closed, &callbacks);
    });
}

fn finish_browser_capture(
    encoder: &BrowserVideoEncoderHandle,
    closed: &Rc<Cell<bool>>,
    callbacks: &ScreenShareCallbacks,
) {
    if closed.replace(true) {
        return;
    }
    if let Err(error) = encoder.close() {
        warn!(%error, "failed to close screen sharing encoder after capture ended");
    }
    (callbacks.on_ended)();
}

fn screen_share_frame_from_video(frame: EncodedVideoFrame) -> EncodedScreenShareFrame {
    EncodedScreenShareFrame {
        sequence: frame.sequence,
        timestamp_us: frame.timestamp_us,
        duration_us: frame.duration_us,
        codec: match frame.codec {
            VideoCodec::Vp9 => ScreenShareCodec::Vp9,
        },
        key_frame: frame.key_frame,
        width: frame.width,
        height: frame.height,
        bytes: frame.bytes,
    }
}

fn screen_share_video_encoding_error(error: VideoEncodingError) -> ScreenShareError {
    if error.is_unsupported() {
        ScreenShareError::with_kind(
            UNSUPPORTED_SCREEN_SHARE_MESSAGE,
            ScreenShareErrorKind::UnsupportedBrowser,
        )
    } else {
        ScreenShareError::new(error.to_string())
    }
}
