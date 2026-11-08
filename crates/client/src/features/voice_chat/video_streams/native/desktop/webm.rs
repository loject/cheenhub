//! Feature-адаптер FFmpeg WebM: ожидание key frame и размеры VP9.

use super::vp9::{self, Dimensions};
use dioxus::logger::tracing::debug;

/// Фрагмент live WebM и обновлённые размеры изображения.
pub(super) struct MuxedFrame {
    /// Заголовок при старте и сброшенный Cluster текущего кадра.
    pub(super) bytes: Vec<u8>,
    /// Размеры текущего ключевого кадра для адаптации плитки.
    pub(super) dimensions: Option<Dimensions>,
}

/// Состояние входящего VP9-потока, упаковываемого статическим FFmpeg.
#[derive(Default)]
pub(super) struct WebmMuxer {
    muxer: Option<cheenhub_video_codec::WebmMuxer>,
    previous_timestamp_us: Option<u64>,
}

impl WebmMuxer {
    /// Пропускает delta до key frame и поздние кадры, остальные упаковывает.
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
        if self.muxer.is_none() && !key_frame {
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
        let starting = self.muxer.is_none();
        if starting {
            let size = dimensions.ok_or("Нет размеров первого VP9-кадра")?;
            self.muxer = Some(cheenhub_video_codec::WebmMuxer::new(
                size.width,
                size.height,
                duration_us,
            )?);
        }
        let mut output = self.muxer.as_mut().ok_or("WebM muxer не создан")?.push(
            bytes,
            timestamp_us,
            duration_us,
            key_frame,
        )?;
        if starting {
            // EBML Void заполняет первый read buffer WebKit/GStreamer, чтобы
            // маленький ключевой кадр не ждал следующих кадров или EOF.
            output.extend([0xec, 0x50, 0x00]);
            output.resize(output.len() + 4096, 0);
        }
        self.previous_timestamp_us = Some(timestamp_us);
        Ok(Some(MuxedFrame {
            bytes: output,
            dimensions,
        }))
    }
}

#[cfg(test)]
mod tests;
