//! Browser-backend демонстрации экрана.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::cell::RefCell;
use std::rc::Rc;

use cheenhub_contracts::video_presets::VideoEncodingSettings;
use dioxus::prelude::{debug, warn};
use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;

mod audio;
mod audio_bindings;
mod frame_loop;
mod lifecycle;
use audio::BrowserScreenAudio;
use frame_loop::{BrowserCaptureFrames, spawn_video_reader};
use lifecycle::{BrowserCaptureLifecycle, BrowserCaptureStopReason};

use crate::features::video_encoding::{
    BrowserVideoEncodingManager, BrowserVideoFrame, BrowserVideoFrameReader, EncodedVideoFrame,
    VideoCodec, VideoEncoderConfig, VideoEncodingAcceleratorKind, VideoEncodingError,
    VideoEncodingManager, VideoFrameEncoder,
};

use super::backend::{
    EncodedScreenShareFrame, ScreenShareBackend, ScreenShareCallbacks, ScreenShareCaptureSource,
    ScreenShareCodec, ScreenShareConfig, ScreenShareError, ScreenShareErrorKind,
    ScreenShareSession, ScreenShareStartRequest, ScreenShareTargetQuality, encoder_config,
    fit_capture_preset,
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
    fn supports_platform_quality_picker(&self) -> bool {
        true
    }

    fn start(
        &self,
        request: ScreenShareStartRequest,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>> {
        async move {
            let target = match request.source {
                ScreenShareCaptureSource::PlatformDefault { target } => target,
                ScreenShareCaptureSource::Selected { .. } => {
                    return Err(ScreenShareError::new(
                        "Выбор физического монитора недоступен в browser backend.",
                    ));
                }
            };
            start_browser_session(request.config, target, callbacks).await
        }
        .boxed_local()
    }
}

struct BrowserScreenShareSession {
    lifecycle: Rc<BrowserCaptureLifecycle>,
}

/// Освобождает полученный поток, если запуск сессии прервался.
struct PendingBrowserCapture {
    stream: web_sys::MediaStream,
    transferred: bool,
}

impl Drop for PendingBrowserCapture {
    fn drop(&mut self) {
        if !self.transferred {
            stop_screen_stream(&self.stream, "startup-drop");
            debug!("released browser screen sharing stream before session activation");
        }
    }
}

impl Drop for BrowserScreenShareSession {
    fn drop(&mut self) {
        self.lifecycle.stop(BrowserCaptureStopReason::Drop);
    }
}

impl ScreenShareSession for BrowserScreenShareSession {
    fn take_audio_frames(
        &self,
    ) -> Option<futures_channel::mpsc::Receiver<super::backend::EncodedScreenAudioFrame>> {
        self.lifecycle.take_audio_frames()
    }
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>> {
        let lifecycle = self.lifecycle.clone();
        async move {
            lifecycle.stop(BrowserCaptureStopReason::Manual);
            Ok(())
        }
        .boxed_local()
    }
}

async fn start_browser_session(
    config: ScreenShareConfig,
    target: Option<ScreenShareTargetQuality>,
    callbacks: ScreenShareCallbacks,
) -> Result<Rc<dyn ScreenShareSession>, ScreenShareError> {
    if config.codec != ScreenShareCodec::Vp9 {
        return Err(ScreenShareError::new(
            "Поддерживается только VP9 демонстрация экрана.",
        ));
    }
    ensure_browser_screen_share_support()?;
    let selected_preset = match target {
        Some(target) => {
            let preset = config.preset_for_target(&target).ok_or_else(|| {
                ScreenShareError::new("Выбранное качество демонстрации недоступно.")
            })?;
            debug!(
                width = preset.width,
                height = preset.height,
                fps = preset.max_fps,
                bitrate_bps = preset.bitrate_bps,
                "browser screen share quality selected"
            );
            Some(preset)
        }
        None => None,
    };

    if callbacks.start_control.is_cancelled() {
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
    }
    let mut pending_capture = PendingBrowserCapture {
        stream: request_screen_stream(selected_preset).await?,
        transferred: false,
    };
    if callbacks.start_control.is_cancelled() {
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
    }
    let track = first_video_track(&pending_capture.stream)?;
    let lifecycle = BrowserCaptureLifecycle::new(
        pending_capture.stream.clone(),
        track.clone(),
        callbacks.clone(),
    );
    pending_capture.transferred = true;
    if callbacks.start_control.is_cancelled() {
        lifecycle.stop(BrowserCaptureStopReason::StartupCancelled);
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
    }
    let selected_audio =
        super::browser_capture::selected_system_audio(&pending_capture.stream, &track);
    log_selected_video_track(&track);
    let (cancel_sender, cancel_receiver) = futures_channel::oneshot::channel();
    let cancel_sender = Rc::new(RefCell::new(Some(cancel_sender)));
    let cleanup_sender = cancel_sender.clone();
    lifecycle.register_pending_start_cleanup(Rc::new(move || {
        if let Some(sender) = cleanup_sender.borrow_mut().take() {
            let _ = sender.send(());
        }
    }));
    let frame_reader_result = BrowserVideoFrameReader::from_stream_cancellable(
        &track,
        &pending_capture.stream,
        Some(cancel_receiver),
    )
    .await;
    lifecycle.clear_pending_start_cleanup();
    let frame_reader = match frame_reader_result {
        Ok(reader) => reader,
        Err(error) => {
            lifecycle.stop(if callbacks.start_control.is_cancelled() {
                BrowserCaptureStopReason::StartupCancelled
            } else {
                BrowserCaptureStopReason::Error
            });
            return Err(screen_share_video_encoding_error(error));
        }
    };
    let frame_reader_handle = frame_reader.handle();
    lifecycle.set_reader(frame_reader_handle);
    if callbacks.start_control.is_cancelled() {
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
    }
    let first_frame = match frame_reader.read().await {
        Ok(Some(frame)) => frame,
        Ok(None) => {
            lifecycle.stop(if callbacks.start_control.is_cancelled() {
                BrowserCaptureStopReason::StartupCancelled
            } else {
                BrowserCaptureStopReason::SourceEnded
            });
            return Err(ScreenShareError::new(
                "Демонстрация завершилась до получения изображения.",
            ));
        }
        Err(error) => {
            lifecycle.stop(if callbacks.start_control.is_cancelled() {
                BrowserCaptureStopReason::StartupCancelled
            } else {
                BrowserCaptureStopReason::Error
            });
            return Err(screen_share_video_encoding_error(error));
        }
    };
    if callbacks.start_control.is_cancelled() {
        first_frame.close();
        lifecycle.stop(BrowserCaptureStopReason::StartupCancelled);
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
    }
    let (source_width, source_height) = first_frame.display_dimensions();
    let capture_anchor = web_time::Instant::now();
    let capture_timestamp_us = first_frame.timestamp_us();
    let preset = fit_capture_preset(
        source_width,
        source_height,
        selected_preset.unwrap_or_else(|| config.preset_for_capture(source_width, source_height)),
    );
    debug!(
        source_width,
        source_height,
        encoded_width = preset.width,
        encoded_height = preset.height,
        "browser screen sharing uses first video frame dimensions within selected quality"
    );
    let encoder_config = encoder_config(preset);
    if let Err(error) = ensure_video_encoder_available(encoder_config.clone()).await {
        first_frame.close();
        lifecycle.stop(if callbacks.start_control.is_cancelled() {
            BrowserCaptureStopReason::StartupCancelled
        } else {
            BrowserCaptureStopReason::Error
        });
        return Err(error);
    }
    if callbacks.start_control.is_cancelled() {
        first_frame.close();
        lifecycle.stop(BrowserCaptureStopReason::StartupCancelled);
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
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
            lifecycle.stop(BrowserCaptureStopReason::Error);
            return Err(screen_share_video_encoding_error(error));
        }
    };
    lifecycle.set_encoder(encoder);
    if callbacks.start_control.is_cancelled() {
        first_frame.close();
        lifecycle.stop(BrowserCaptureStopReason::StartupCancelled);
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
    }

    let audio = if let Some(audio_track) = selected_audio {
        use futures_util::future::{Either, select};
        let (cancel_sender, cancel_receiver) = futures_channel::oneshot::channel();
        let cancel_sender = Rc::new(RefCell::new(Some(cancel_sender)));
        let cleanup_sender = cancel_sender.clone();
        lifecycle.register_pending_start_cleanup(Rc::new(move || {
            if let Some(sender) = cleanup_sender.borrow_mut().take() {
                let _ = sender.send(());
            }
        }));
        let capture = BrowserScreenAudio::start(
            audio_track.clone(),
            track.clone(),
            callbacks.clone(),
            capture_timestamp_us,
            capture_anchor,
        )
        .boxed_local();
        let timeout = crate::features::runtime::sleep_ms(2_000).boxed_local();
        let startup = select(capture, timeout).boxed_local();
        let cancelled = async move {
            let _ = cancel_receiver.await;
            Err(ScreenShareError::new("Запуск звука демонстрации отменён."))
        }
        .boxed_local();
        let result = match select(startup, cancelled).await {
            Either::Left((result, _)) => match result {
                Either::Left((result, _)) => result,
                Either::Right(_) => Err(ScreenShareError::new(
                    "Истекло время включения звука демонстрации.",
                )),
            },
            Either::Right((result, _)) => result,
        };
        lifecycle.clear_pending_start_cleanup();
        if callbacks.start_control.is_cancelled() {
            lifecycle.stop(BrowserCaptureStopReason::StartupCancelled);
            return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
        }
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
    lifecycle.set_audio(audio.clone());
    if callbacks.start_control.is_cancelled() {
        first_frame.close();
        lifecycle.stop(BrowserCaptureStopReason::StartupCancelled);
        return Err(ScreenShareError::new("Запуск демонстрации был отменён."));
    }
    let Some(encoder_handle) = lifecycle.encoder_handle() else {
        first_frame.close();
        lifecycle.stop(BrowserCaptureStopReason::Error);
        return Err(ScreenShareError::new(
            "Кодировщик демонстрации закрыт во время запуска.",
        ));
    };
    spawn_video_reader(
        BrowserCaptureFrames {
            reader: frame_reader,
            first_frame,
            lifecycle: lifecycle.clone(),
        },
        encoder_handle,
        callbacks,
        config,
        selected_preset,
        encoder_config,
    );

    Ok(Rc::new(BrowserScreenShareSession { lifecycle }))
}

async fn ensure_video_encoder_available(
    encoder_config: VideoEncoderConfig,
) -> Result<(), ScreenShareError> {
    let manager = BrowserVideoEncodingManager;
    let available = match manager.available_accelerators(encoder_config).await {
        Ok(available) => available,
        Err(error) => return Err(screen_share_video_encoding_error(error)),
    };
    if !available
        .iter()
        .any(|encoder| encoder.kind == VideoEncodingAcceleratorKind::WebCodecs)
    {
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
