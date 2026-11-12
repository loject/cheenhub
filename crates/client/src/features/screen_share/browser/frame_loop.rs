//! Чтение, ограничение частоты и кодирование кадров демонстрации.

use super::*;
use crate::features::screen_share::backend;
use crate::features::video_encoding::{BrowserVideoEncoderHandle, VideoFrameRateGate};
use wasm_bindgen_futures::spawn_local;

/// Данные источника кадров и средства его освобождения.
pub(super) struct BrowserCaptureFrames {
    pub(super) reader: BrowserVideoFrameReader,
    pub(super) first_frame: BrowserVideoFrame,
    pub(super) lifecycle: Rc<BrowserCaptureLifecycle>,
}

pub(super) fn spawn_video_reader(
    source: BrowserCaptureFrames,
    encoder: BrowserVideoEncoderHandle,
    callbacks: ScreenShareCallbacks,
    config: ScreenShareConfig,
    selected_preset: Option<VideoEncodingSettings>,
    mut encoder_config: VideoEncoderConfig,
) {
    spawn_local(async move {
        let BrowserCaptureFrames {
            reader,
            first_frame,
            lifecycle,
        } = source;
        let mut first_frame = Some(first_frame);
        let mut frame_sequence = 0_u64;
        let mut frame_rate_gate = VideoFrameRateGate::new(encoder_config.frame_rate);
        while !lifecycle.is_stopped() {
            let frame = match first_frame.take() {
                Some(frame) => frame,
                None => match reader.read().await {
                    Ok(Some(frame)) => frame,
                    Ok(None) => {
                        debug!(
                            stopped = lifecycle.is_stopped(),
                            "browser screen sharing frame source ended"
                        );
                        break;
                    }
                    Err(error) => {
                        if !lifecycle.is_stopped() {
                            warn!(%error, "failed to read screen sharing frame");
                            lifecycle.stop(BrowserCaptureStopReason::Error);
                            (callbacks.on_error)(screen_share_video_encoding_error(error));
                        }
                        break;
                    }
                },
            };
            if lifecycle.is_stopped() {
                debug!("discarded screen sharing frame received after capture stopped");
                frame.close();
                break;
            }
            let (source_width, source_height) = frame.display_dimensions();
            let preset = fit_capture_preset(
                source_width,
                source_height,
                selected_preset
                    .unwrap_or_else(|| config.preset_for_capture(source_width, source_height)),
            );
            let next_config = backend::encoder_config(preset);
            if next_config != encoder_config {
                debug!(
                    source_width,
                    source_height,
                    encoded_width = next_config.width,
                    encoded_height = next_config.height,
                    "screen sharing video frame dimensions changed; updating encoder"
                );
                if let Err(error) = encoder.reconfigure(&next_config).await {
                    if !lifecycle.is_stopped() {
                        warn!(%error, "failed to reconfigure screen sharing encoder");
                        lifecycle.stop(BrowserCaptureStopReason::Error);
                        (callbacks.on_error)(screen_share_video_encoding_error(error));
                    }
                    frame.close();
                    break;
                }
                if lifecycle.is_stopped() {
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
                lifecycle.stop(BrowserCaptureStopReason::Error);
                (callbacks.on_error)(screen_share_video_encoding_error(error));
                frame.close();
                break;
            }
            frame.close();
        }

        lifecycle.stop(BrowserCaptureStopReason::SourceEnded);
    });
}
