//! Адаптер FFmpeg VP9 с программным резервом для Windows Graphics Capture.

use super::frames::I420Frame;
use crate::features::video_encoding::{EncodedVideoFrame, VideoCodec};
use cheenhub_video_codec::{EncodedPacket, Vp9Encoder};

const KEY_FRAME_INTERVAL_US: u64 = 2_000_000;

/// Encoder одного capture worker; FFmpeg-контекст освобождается вместе с ним.
pub(super) struct WindowsVp9Encoder {
    encoder: Option<Vp9Encoder>,
    width: u32,
    height: u32,
    duration_us: u32,
    sequence: u64,
    last_key_frame_us: Option<u64>,
    last_timestamp_us: Option<u64>,
    awaiting_first_key_frame: bool,
}

impl WindowsVp9Encoder {
    /// Открывает encoder выбранного пресета с резервным программным VP9.
    pub(super) fn new(width: u32, height: u32, bitrate_bps: u32, fps: u32) -> Result<Self, String> {
        Ok(Self {
            encoder: Some(Vp9Encoder::new(width, height, fps, bitrate_bps)?),
            width,
            height,
            duration_us: 1_000_000 / fps,
            sequence: 0,
            last_key_frame_us: None,
            last_timestamp_us: None,
            awaiting_first_key_frame: true,
        })
    }

    /// Завершает прежний контекст и возвращает отложенные пакеты до смены размеров.
    pub(super) fn finish_for_resize(&mut self) -> Result<Vec<EncodedVideoFrame>, String> {
        let old = self.encoder.take().ok_or("VP9 encoder уже закрыт")?;
        let packets = old.finish()?;
        self.wrap_packets(packets)
    }

    /// Открывает новый контекст после завершения старого; ошибка завершает захват.
    pub(super) fn open_after_resize(
        &mut self,
        width: u32,
        height: u32,
        bitrate_bps: u32,
        fps: u32,
    ) -> Result<(), String> {
        if self.encoder.is_some() {
            return Err("Прежний VP9 encoder ещё открыт".to_owned());
        }
        let replacement = Vp9Encoder::new(width, height, fps, bitrate_bps)?;
        self.encoder = Some(replacement);
        self.width = width;
        self.height = height;
        self.duration_us = 1_000_000 / fps;
        self.last_key_frame_us = None;
        self.awaiting_first_key_frame = true;
        Ok(())
    }

    /// Передаёт I420 и возвращает готовые пакеты существующего медиаконтракта.
    pub(super) fn encode(
        &mut self,
        frame: &I420Frame,
        timestamp_us: u64,
    ) -> Result<Vec<EncodedVideoFrame>, String> {
        let key_frame = should_force_key_frame(self.last_key_frame_us, timestamp_us);
        let packets = self
            .encoder
            .as_mut()
            .ok_or("VP9 encoder уже закрыт")?
            .encode(&frame.bytes, timestamp_us, key_frame)?;
        if key_frame {
            self.last_key_frame_us = Some(timestamp_us);
        }
        self.wrap_packets(packets)
    }

    fn wrap_packets(
        &mut self,
        packets: Vec<EncodedPacket>,
    ) -> Result<Vec<EncodedVideoFrame>, String> {
        let mut output = Vec::with_capacity(packets.len());
        for packet in packets {
            if self.awaiting_first_key_frame && !packet.key_frame {
                return Err("VP9 encoder не выдал ключевой кадр после запуска".to_owned());
            }
            self.awaiting_first_key_frame = false;
            if self
                .last_timestamp_us
                .is_some_and(|last| packet.timestamp_us < last)
            {
                return Err("FFmpeg вернул VP9-пакеты с нарушением порядка времени".to_owned());
            }
            self.last_timestamp_us = Some(packet.timestamp_us);
            let sequence = self.sequence;
            self.sequence = self.sequence.saturating_add(1);
            output.push(EncodedVideoFrame {
                sequence,
                timestamp_us: packet.timestamp_us,
                duration_us: self.duration_us,
                codec: VideoCodec::Vp9,
                key_frame: packet.key_frame,
                width: self.width,
                height: self.height,
                bytes: packet.bytes,
            });
        }
        Ok(output)
    }
}

fn should_force_key_frame(last_key_frame_us: Option<u64>, timestamp_us: u64) -> bool {
    last_key_frame_us.is_none_or(|last| timestamp_us.saturating_sub(last) >= KEY_FRAME_INTERVAL_US)
}

#[cfg(test)]
mod tests;
