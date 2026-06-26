//! Browser-backend камеры.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::warn;
use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::MediaStream;

use crate::features::video_encoding::{
    BrowserVideoEncoder, BrowserVideoEncoderHandle, BrowserVideoEncodingManager,
    BrowserVideoFrameReader, EncodedVideoFrame, VideoCodec, VideoEncoderConfig,
    VideoEncodingAcceleratorKind, VideoEncodingError, VideoEncodingManager, VideoFrameEncoder,
};

use super::backend::{
    CameraBackend, CameraCallbacks, CameraCodec, CameraConfig, CameraError, CameraErrorKind,
    CameraSession, EncodedCameraFrame,
};
use super::browser_capture::{
    first_video_track, log_selected_video_track, request_camera_stream, video_track_settings,
};

const KEY_FRAME_INTERVAL_SECONDS: u32 = 2;
const UNSUPPORTED_CAMERA_MESSAGE: &str = concat!(
    "Этот браузер не поддерживает камеру в CheenHub. ",
    "Воспользуйтесь браузером на базе Chromium или нативным клиентом."
);

/// Реализация браузерной камеры на основе `getUserMedia` и WebCodecs.
pub(crate) struct WebCameraBackend;

impl CameraBackend for WebCameraBackend {
    fn start(
        &self,
        config: CameraConfig,
        callbacks: CameraCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn CameraSession>, CameraError>> {
        async move { start_browser_session(config, callbacks).await }.boxed_local()
    }
}

struct BrowserCameraSession {
    encoder: BrowserVideoEncoder,
    stream: MediaStream,
    track: web_sys::MediaStreamTrack,
    closed: Rc<Cell<bool>>,
}

impl CameraSession for BrowserCameraSession {
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), CameraError>> {
        let encoder = self.encoder.handle();
        let stream = self.stream.clone();
        let track = self.track.clone();
        let closed = self.closed.clone();
        async move {
            if closed.replace(true) {
                return Ok(());
            }
            track.stop();
            stop_media_stream(&stream);
            encoder.close().map_err(camera_video_encoding_error)?;
            Ok(())
        }
        .boxed_local()
    }
}

async fn start_browser_session(
    config: CameraConfig,
    callbacks: CameraCallbacks,
) -> Result<Rc<dyn CameraSession>, CameraError> {
    if config.codec != CameraCodec::Vp9 {
        return Err(CameraError::new("Поддерживается только VP9 камера."));
    }
    ensure_browser_camera_support()?;

    let stream = request_camera_stream(&config).await?;
    let track = match first_video_track(&stream) {
        Ok(track) => track,
        Err(error) => {
            stop_media_stream(&stream);
            return Err(error);
        }
    };
    log_selected_video_track(&track);
    let settings = video_track_settings(&track);
    let width = settings.width.unwrap_or(config.width).max(1);
    let height = settings.height.unwrap_or(config.height).max(1);
    let frame_rate = settings.frame_rate.unwrap_or(config.frame_rate).max(1);
    let encoder_config = VideoEncoderConfig::vp9(width, height, frame_rate, config.bitrate_bps);
    ensure_video_encoder_available(&stream, encoder_config.clone()).await?;

    let output_on_frame = callbacks.on_frame.clone();
    let on_video_frame = Rc::new(move |frame: EncodedVideoFrame| {
        output_on_frame(camera_frame_from_video(frame));
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
            stop_media_stream(&stream);
            return Err(camera_video_encoding_error(error));
        }
    };

    let closed = Rc::new(Cell::new(false));
    let key_frame_interval_frames = frame_rate.saturating_mul(KEY_FRAME_INTERVAL_SECONDS).max(1);
    spawn_video_reader(
        stream.clone(),
        track.clone(),
        encoder.handle(),
        closed.clone(),
        callbacks,
        key_frame_interval_frames,
    );

    Ok(Rc::new(BrowserCameraSession {
        encoder,
        stream,
        track,
        closed,
    }))
}

async fn ensure_video_encoder_available(
    stream: &MediaStream,
    encoder_config: VideoEncoderConfig,
) -> Result<(), CameraError> {
    let manager = BrowserVideoEncodingManager;
    let available = match manager.available_accelerators(encoder_config).await {
        Ok(available) => available,
        Err(error) => {
            stop_media_stream(stream);
            return Err(camera_video_encoding_error(error));
        }
    };
    if !available
        .iter()
        .any(|encoder| encoder.kind == VideoEncodingAcceleratorKind::WebCodecs)
    {
        stop_media_stream(stream);
        return Err(CameraError::new(
            "Браузер не поддерживает кодирование камеры в VP9.",
        ));
    }

    Ok(())
}

fn ensure_browser_camera_support() -> Result<(), CameraError> {
    let manager = BrowserVideoEncodingManager;
    if !manager.browser_track_pipeline_available() {
        return Err(CameraError::with_kind(
            UNSUPPORTED_CAMERA_MESSAGE,
            CameraErrorKind::UnsupportedBrowser,
        ));
    }

    Ok(())
}

fn spawn_video_reader(
    stream: MediaStream,
    track: web_sys::MediaStreamTrack,
    encoder: BrowserVideoEncoderHandle,
    closed: Rc<Cell<bool>>,
    callbacks: CameraCallbacks,
    key_frame_interval_frames: u32,
) {
    spawn_local(async move {
        let frame_sequence = Rc::new(Cell::new(0_u64));
        let reader = match BrowserVideoFrameReader::from_track(&track) {
            Ok(reader) => reader,
            Err(error) => {
                warn!(%error, "failed to create camera video frame reader");
                finish_browser_capture(&stream, &encoder, &closed, &callbacks);
                return;
            }
        };

        while !closed.get() {
            let frame = match reader.read().await {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(error) => {
                    warn!(%error, "failed to read camera frame");
                    break;
                }
            };
            let sequence = frame_sequence.get();
            frame_sequence.set(sequence.saturating_add(1));
            let key_frame = sequence.is_multiple_of(u64::from(key_frame_interval_frames));
            if let Err(error) = encoder.encode(&frame, key_frame) {
                warn!(%error, "failed to encode camera frame");
                frame.close();
                break;
            }
            frame.close();
        }

        finish_browser_capture(&stream, &encoder, &closed, &callbacks);
    });
}

fn finish_browser_capture(
    stream: &MediaStream,
    encoder: &BrowserVideoEncoderHandle,
    closed: &Rc<Cell<bool>>,
    callbacks: &CameraCallbacks,
) {
    if closed.replace(true) {
        return;
    }
    stop_media_stream(stream);
    if let Err(error) = encoder.close() {
        warn!(%error, "failed to close camera encoder after capture ended");
    }
    (callbacks.on_ended)();
}

fn camera_frame_from_video(frame: EncodedVideoFrame) -> EncodedCameraFrame {
    EncodedCameraFrame {
        sequence: frame.sequence,
        timestamp_us: frame.timestamp_us,
        duration_us: frame.duration_us,
        codec: match frame.codec {
            VideoCodec::Vp9 => CameraCodec::Vp9,
        },
        key_frame: frame.key_frame,
        width: frame.width,
        height: frame.height,
        bytes: frame.bytes,
    }
}

fn camera_video_encoding_error(error: VideoEncodingError) -> CameraError {
    if error.is_unsupported() {
        CameraError::with_kind(
            UNSUPPORTED_CAMERA_MESSAGE,
            CameraErrorKind::UnsupportedBrowser,
        )
    } else {
        CameraError::new(error.to_string())
    }
}

fn stop_media_stream(stream: &MediaStream) {
    let tracks = stream.get_video_tracks();
    for i in 0..tracks.length() {
        if let Ok(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
            track.stop();
        }
    }
}
