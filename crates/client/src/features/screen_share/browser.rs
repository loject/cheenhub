//! Browser-backend демонстрации экрана.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::{debug, warn};
use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;
use wasm_bindgen_futures::spawn_local;

mod audio;
mod audio_bindings;
use audio::BrowserScreenAudio;

use crate::features::video_encoding::{
    BrowserVideoEncoder, BrowserVideoEncoderHandle, BrowserVideoEncodingManager, BrowserVideoFrame,
    BrowserVideoFrameReader, BrowserVideoFrameReaderHandle, EncodedVideoFrame, VideoCodec,
    VideoEncoderConfig, VideoEncodingAcceleratorKind, VideoEncodingError, VideoEncodingManager,
    VideoFrameEncoder, VideoFrameRateGate,
};

use super::backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCaptureSource,
    ScreenShareCodec, ScreenShareConfig, ScreenShareError, ScreenShareErrorKind,
    ScreenShareSession, ScreenShareStartRequest, fit_capture_preset,
};
use super::browser_capture::{
    first_video_track, log_selected_video_track, request_screen_stream, stop_screen_stream,
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
        request: ScreenShareStartRequest,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        async move {
            if matches!(request.source, ScreenShareCaptureSource::Selected { .. }) {
                return Err(ScreenShareError::new(
                    "Выбор физического монитора недоступен в browser backend.",
                ));
            }
            start_browser_session(request.config, callbacks).await
        }
        .boxed_local()
    }
}

struct BrowserScreenShareSession {
    audio: Option<Rc<BrowserScreenAudio>>,
    encoder: BrowserVideoEncoder,
    track: web_sys::MediaStreamTrack,
    frame_reader: BrowserVideoFrameReaderHandle,
    closed: Rc<Cell<bool>>,
}

/// Освобождает полученный поток, если запуск сессии прервался.
struct PendingBrowserCapture {
    stream: web_sys::MediaStream,
    transferred: bool,
}

impl Drop for PendingBrowserCapture {
    fn drop(&mut self) {
        if !self.transferred {
            stop_screen_stream(&self.stream);
            debug!("released browser screen sharing stream before session activation");
        }
    }
}

impl Drop for BrowserScreenShareSession {
    fn drop(&mut self) {
        if let Some(audio) = &self.audio {
            audio.stop();
        }
        if self.closed.replace(true) {
            return;
        }
        self.frame_reader.stop();
        self.track.stop();
        if let Err(error) = self.encoder.close() {
            warn!(%error, "failed to close screen sharing encoder on session drop");
        }
        debug!("released browser screen sharing capture on session drop");
    }
}

impl ScreenShareSession for BrowserScreenShareSession {
    fn take_audio_frames(
        &self,
    ) -> Option<futures_channel::mpsc::Receiver<super::backend::EncodedScreenAudioFrame>> {
        self.audio.as_ref()?.take_frames()
    }
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>> {
        if let Some(audio) = &self.audio {
            audio.stop();
        }
        let encoder = self.encoder.handle();
        let track = self.track.clone();
        let frame_reader = self.frame_reader.clone();
        let closed = self.closed.clone();
        async move {
            if closed.replace(true) {
                return Ok(());
            }
            debug!("browser screen sharing explicitly stopped; closing capture and encoder");
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

    let mut pending_capture = PendingBrowserCapture {
        stream: request_screen_stream().await?,
        transferred: false,
    };
    let track = first_video_track(&pending_capture.stream)?;
    let selected_audio =
        super::browser_capture::selected_system_audio(&pending_capture.stream, &track);
    log_selected_video_track(&track);
    let frame_reader =
        match BrowserVideoFrameReader::from_stream(&track, &pending_capture.stream).await {
            Ok(reader) => reader,
            Err(error) => {
                track.stop();
                return Err(screen_share_video_encoding_error(error));
            }
        };
    let first_frame = match frame_reader.read().await {
        Ok(Some(frame)) => frame,
        Ok(None) => {
            track.stop();
            return Err(ScreenShareError::new(
                "Демонстрация завершилась до получения изображения.",
            ));
        }
        Err(error) => {
            track.stop();
            return Err(screen_share_video_encoding_error(error));
        }
    };
    let (source_width, source_height) = first_frame.display_dimensions();
    let capture_anchor = web_time::Instant::now();
    let capture_timestamp_us = first_frame.timestamp_us();
    let preset = fit_capture_preset(
        source_width,
        source_height,
        config.preset_for_capture(source_width, source_height),
    );
    debug!(
        source_width,
        source_height,
        encoded_width = preset.width,
        encoded_height = preset.height,
        "browser screen sharing uses first video frame dimensions within selected quality"
    );
    let encoder_config = VideoEncoderConfig::vp9(
        preset.width,
        preset.height,
        preset.max_fps,
        preset.bitrate_bps,
    );
    if let Err(error) = ensure_video_encoder_available(&track, encoder_config.clone()).await {
        first_frame.close();
        return Err(error);
    }

    let output_on_frame = callbacks.on_frame.clone();
    let on_video_frame = Rc::new(move |frame: EncodedVideoFrame| {
        output_on_frame(screen_share_frame_from_video(frame));
    });
    let manager = BrowserVideoEncodingManager;
    let encoder = match manager
        .create_encoder(
            VideoEncodingAcceleratorKind::WebCodecs,
            encoder_config.clone(),
            on_video_frame,
        )
        .await
    {
        Ok(encoder) => encoder,
        Err(error) => {
            first_frame.close();
            track.stop();
            return Err(screen_share_video_encoding_error(error));
        }
    };
    let frame_reader_handle = frame_reader.handle();

    let audio = if let Some(audio_track) = selected_audio {
        use futures_util::future::{Either, select};
        let capture = BrowserScreenAudio::start(
            audio_track.clone(),
            track.clone(),
            callbacks.clone(),
            capture_timestamp_us,
            capture_anchor,
        )
        .boxed_local();
        let timeout = crate::features::runtime::sleep_ms(2_000).boxed_local();
        let result = match select(capture, timeout).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => Err(ScreenShareError::new(
                "Истекло время включения звука демонстрации.",
            )),
        };
        match result {
            Ok(audio) => Some(audio),
            Err(error) => {
                audio_track.stop();
                warn!(%error, "screen sharing continues without system audio");
                (callbacks.on_audio_state)(super::backend::ScreenShareAudioState::Unavailable);
                None
            }
        }
    } else {
        None
    };

    let closed = Rc::new(Cell::new(false));
    pending_capture.transferred = true;
    spawn_video_reader(
        BrowserCaptureFrames {
            audio: audio.clone(),
            reader: frame_reader,
            first_frame,
            track: track.clone(),
            stop_handle: frame_reader_handle.clone(),
        },
        encoder.handle(),
        closed.clone(),
        callbacks,
        config,
        encoder_config,
    );

    Ok(Rc::new(BrowserScreenShareSession {
        audio,
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

/// Данные источника кадров и средства его освобождения.
struct BrowserCaptureFrames {
    audio: Option<Rc<BrowserScreenAudio>>,
    reader: BrowserVideoFrameReader,
    first_frame: BrowserVideoFrame,
    track: web_sys::MediaStreamTrack,
    stop_handle: BrowserVideoFrameReaderHandle,
}

fn spawn_video_reader(
    source: BrowserCaptureFrames,
    encoder: BrowserVideoEncoderHandle,
    closed: Rc<Cell<bool>>,
    callbacks: ScreenShareCallbacks,
    config: ScreenShareConfig,
    mut encoder_config: VideoEncoderConfig,
) {
    spawn_local(async move {
        let BrowserCaptureFrames {
            audio,
            reader,
            first_frame,
            track,
            stop_handle,
        } = source;
        let mut first_frame = Some(first_frame);
        let mut frame_sequence = 0_u64;
        let mut frame_rate_gate = VideoFrameRateGate::new(encoder_config.frame_rate);
        while !closed.get() {
            let frame = match first_frame.take() {
                Some(frame) => frame,
                None => match reader.read().await {
                    Ok(Some(frame)) => frame,
                    Ok(None) => {
                        debug!(
                            stopped = closed.get(),
                            "browser screen sharing frame source ended"
                        );
                        break;
                    }
                    Err(error) => {
                        if !closed.get() {
                            warn!(%error, "failed to read screen sharing frame");
                            (callbacks.on_error)(screen_share_video_encoding_error(error));
                        }
                        break;
                    }
                },
            };
            if closed.get() {
                debug!("discarded screen sharing frame received after capture stopped");
                frame.close();
                break;
            }
            let (source_width, source_height) = frame.display_dimensions();
            let preset = fit_capture_preset(
                source_width,
                source_height,
                config.preset_for_capture(source_width, source_height),
            );
            let next_config = VideoEncoderConfig::vp9(
                preset.width,
                preset.height,
                preset.max_fps,
                preset.bitrate_bps,
            );
            if next_config != encoder_config {
                debug!(
                    source_width,
                    source_height,
                    encoded_width = next_config.width,
                    encoded_height = next_config.height,
                    "screen sharing video frame dimensions changed; updating encoder"
                );
                if let Err(error) = encoder.reconfigure(&next_config).await {
                    if !closed.get() {
                        warn!(%error, "failed to reconfigure screen sharing encoder");
                        (callbacks.on_error)(screen_share_video_encoding_error(error));
                    }
                    frame.close();
                    break;
                }
                if closed.get() {
                    frame.close();
                    break;
                }
                frame_rate_gate = VideoFrameRateGate::new(next_config.frame_rate);
                frame_sequence = 0;
                encoder_config = next_config;
            }
            if !frame_rate_gate.accept(frame.timestamp_us()) {
                frame.close();
                continue;
            }
            let sequence = frame_sequence;
            frame_sequence = sequence.saturating_add(1);
            let key_frame_interval_frames = encoder_config
                .frame_rate
                .saturating_mul(KEY_FRAME_INTERVAL_SECONDS)
                .max(1);
            let key_frame = sequence.is_multiple_of(u64::from(key_frame_interval_frames));
            if let Err(error) = encoder.encode(&frame, key_frame) {
                warn!(%error, "failed to encode screen sharing frame");
                (callbacks.on_error)(screen_share_video_encoding_error(error));
                frame.close();
                break;
            }
            frame.close();
        }

        if let Some(audio) = &audio {
            audio.stop();
        }
        finish_browser_capture(&track, &stop_handle, &encoder, &closed, &callbacks);
    });
}

fn finish_browser_capture(
    track: &web_sys::MediaStreamTrack,
    frame_reader: &BrowserVideoFrameReaderHandle,
    encoder: &BrowserVideoEncoderHandle,
    closed: &Rc<Cell<bool>>,
    callbacks: &ScreenShareCallbacks,
) {
    if closed.replace(true) {
        return;
    }
    frame_reader.stop();
    track.stop();
    debug!("released browser screen sharing track after frame reader ended");
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
