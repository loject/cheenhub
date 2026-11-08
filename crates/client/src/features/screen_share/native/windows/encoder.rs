//! Адаптер FFmpeg VP9 с программным резервом для Windows Graphics Capture.

use super::frames::I420Frame;
use crate::features::video_encoding::{EncodedVideoFrame, VideoCodec};
use cheenhub_video_codec::Vp9Encoder;

const KEY_FRAME_INTERVAL_US: u64 = 2_000_000;

/// Encoder одного capture worker; FFmpeg-контекст освобождается вместе с ним.
pub(super) struct WindowsVp9Encoder {
    encoder: Vp9Encoder,
    width: u32,
    height: u32,
    duration_us: u32,
    sequence: u64,
    last_key_frame_us: Option<u64>,
}

impl WindowsVp9Encoder {
    /// Открывает encoder выбранного пресета с резервным программным VP9.
    pub(super) fn new(width: u32, height: u32, bitrate_bps: u32, fps: u32) -> Result<Self, String> {
        Ok(Self {
            encoder: Vp9Encoder::new(width, height, fps, bitrate_bps)?,
            width,
            height,
            duration_us: 1_000_000 / fps,
            sequence: 0,
            last_key_frame_us: None,
        })
    }

    /// Передаёт I420 и возвращает готовые пакеты существующего медиаконтракта.
    pub(super) fn encode(
        &mut self,
        frame: &I420Frame,
        timestamp_us: u64,
    ) -> Result<Vec<EncodedVideoFrame>, String> {
        let key_frame = should_force_key_frame(self.last_key_frame_us, timestamp_us);
        let packets = self.encoder.encode(&frame.bytes, timestamp_us, key_frame)?;
        if key_frame {
            self.last_key_frame_us = Some(timestamp_us);
        }
        Ok(packets
            .into_iter()
            .map(|packet| {
                let sequence = self.sequence;
                self.sequence = self.sequence.saturating_add(1);
                EncodedVideoFrame {
                    sequence,
                    timestamp_us: packet.timestamp_us,
                    duration_us: self.duration_us,
                    codec: VideoCodec::Vp9,
                    key_frame: packet.key_frame,
                    width: self.width,
                    height: self.height,
                    bytes: packet.bytes,
                }
            })
            .collect())
    }
}

fn should_force_key_frame(last_key_frame_us: Option<u64>, timestamp_us: u64) -> bool {
    last_key_frame_us.is_none_or(|last| timestamp_us.saturating_sub(last) >= KEY_FRAME_INTERVAL_US)
}

#[cfg(test)]
mod tests;
