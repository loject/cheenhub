//! Потоковая упаковка готовых VP9-кадров в WebM без перекодирования.

use super::vp9::{self, Dimensions};
use dioxus::logger::tracing::debug;

/// Готовый фрагмент WebM и отображаемые размеры ключевого кадра.
pub(super) struct MuxedFrame {
    /// EBML header первого кадра и законченный Cluster текущего кадра.
    pub(super) bytes: Vec<u8>,
    /// Новые размеры, если текущий кадр является ключевым.
    pub(super) dimensions: Option<Dimensions>,
}

/// Состояние одного WebM Segment; временная шкала начинается с первого key frame.
#[derive(Default)]
pub(super) struct WebmMuxer {
    origin_us: Option<u64>,
    previous_timestamp_us: Option<u64>,
}

impl WebmMuxer {
    /// Упаковывает кадр, пропуская delta-кадры до первого ключевого.
    pub(super) fn push(
        &mut self,
        bytes: &[u8],
        timestamp_us: u64,
        duration_us: u32,
        key_frame: bool,
    ) -> Result<Option<MuxedFrame>, String> {
        if bytes.is_empty() {
            return Err("Пустой VP9-кадр".into());
        }
        if self.origin_us.is_none() && !key_frame {
            return Ok(None);
        }
        if self
            .previous_timestamp_us
            .is_some_and(|previous| timestamp_us < previous)
        {
            debug!(
                timestamp_us,
                previous_timestamp_us = self.previous_timestamp_us,
                "dropping late VP9 frame before WebM muxing"
            );
            return Ok(None);
        }
        let dimensions = key_frame.then(|| vp9::dimensions(bytes)).transpose()?;
        let mut output = Vec::new();
        let starting = self.origin_us.is_none();
        if starting {
            let dimensions = dimensions.ok_or("Нет размеров первого кадра VP9")?;
            output = header(dimensions, duration_us);
            self.origin_us = Some(timestamp_us);
        }
        let timecode = (timestamp_us - self.origin_us.unwrap_or(timestamp_us)) / 1000;
        let mut cluster = uint(&[0xe7], timecode);
        let mut block = vec![0x81, 0, 0, if key_frame { 0x80 } else { 0 }];
        block.extend_from_slice(bytes);
        cluster.extend(element(&[0xa3], &block));
        output.extend(element(&[0x1f, 0x43, 0xb6, 0x75], &cluster));
        if starting {
            // Заполняет первый read buffer GStreamer, чтобы маленький key frame
            // не ожидал следующих кадров или EOF. Void не меняет медиаданные.
            output.extend(element(&[0xec], &[0; 4096]));
        }
        self.previous_timestamp_us = Some(timestamp_us);
        Ok(Some(MuxedFrame {
            bytes: output,
            dimensions,
        }))
    }
}

fn header(dimensions: Dimensions, duration_us: u32) -> Vec<u8> {
    let mut ebml = uint(&[0x42, 0x86], 1);
    ebml.extend(uint(&[0x42, 0xf7], 1));
    ebml.extend(uint(&[0x42, 0xf2], 4));
    ebml.extend(uint(&[0x42, 0xf3], 8));
    ebml.extend(element(&[0x42, 0x82], b"webm"));
    ebml.extend(uint(&[0x42, 0x87], 4));
    ebml.extend(uint(&[0x42, 0x85], 2));
    let mut output = element(&[0x1a, 0x45, 0xdf, 0xa3], &ebml);
    // Неизвестная длина Segment позволяет воспроизводить незавершённый live-поток.
    output.extend([
        0x18, 0x53, 0x80, 0x67, 0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ]);
    let mut info = uint(&[0x2a, 0xd7, 0xb1], 1_000_000);
    info.extend(element(&[0x4d, 0x80], b"CheenHub"));
    info.extend(element(&[0x57, 0x41], b"CheenHub"));
    output.extend(element(&[0x15, 0x49, 0xa9, 0x66], &info));

    let mut track = uint(&[0xd7], 1);
    track.extend(uint(&[0x73, 0xc5], 1));
    track.extend(uint(&[0x83], 1));
    track.extend(uint(&[0x9c], 0));
    track.extend(element(&[0x86], b"V_VP9"));
    track.extend(uint(
        &[0x23, 0xe3, 0x83],
        u64::from(duration_us.max(1)) * 1000,
    ));
    let mut video = uint(&[0xb0], u64::from(dimensions.width));
    video.extend(uint(&[0xba], u64::from(dimensions.height)));
    video.extend(uint(&[0x54, 0xb0], u64::from(dimensions.display_width)));
    video.extend(uint(&[0x54, 0xba], u64::from(dimensions.display_height)));
    track.extend(element(&[0xe0], &video));
    output.extend(element(
        &[0x16, 0x54, 0xae, 0x6b],
        &element(&[0xae], &track),
    ));
    output
}

fn uint(id: &[u8], value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let leading = bytes.iter().position(|byte| *byte != 0).unwrap_or(7);
    element(id, &bytes[leading..])
}

fn element(id: &[u8], data: &[u8]) -> Vec<u8> {
    let size = data.len() as u64;
    let width = (1..=8)
        .find(|width| size < (1_u64 << (7 * width)) - 1)
        .unwrap_or(8);
    let encoded = (size | (1_u64 << (7 * width))).to_be_bytes();
    let mut output = Vec::with_capacity(id.len() + width + data.len());
    output.extend_from_slice(id);
    output.extend_from_slice(&encoded[8 - width..]);
    output.extend_from_slice(data);
    output
}

#[cfg(test)]
mod tests;
