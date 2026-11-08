//! Потоковый WebM через ffmpeg-next с ограничением памяти выходного AVIO.

use crate::check;
use ffmpeg_next::{Dictionary, Packet, ffi, format, packet};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Default)]
struct OutputBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for OutputBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut buffer = self
            .0
            .lock()
            .map_err(|_| io::Error::other("WebM buffer poisoned"))?;
        if bytes.len() > MAX_OUTPUT_BYTES.saturating_sub(buffer.len()) {
            return Err(io::Error::other("WebM output exceeds memory budget"));
        }
        buffer.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Один live WebM Segment с относительными timestamp и ограничением памяти.
///
/// Первый пакет должен быть ключевым. Входящие VP9-пакеты не перекодируются.
pub struct WebmMuxer {
    output: format::context::Output,
    buffer: OutputBuffer,
    origin_us: Option<u64>,
    previous_timestamp: Option<u64>,
}

impl WebmMuxer {
    /// Создаёт live WebM для указанного размера и длительности кадра в мкс.
    ///
    /// Нулевая длительность означает отсутствие оценки FPS: частота дорожки
    /// не задаётся, воспроизведение опирается на timestamp входящих пакетов.
    ///
    /// # Errors
    /// Отклоняет неверные размеры/длительность и ошибки FFmpeg.
    pub fn new(width: u32, height: u32, duration_us: u32) -> Result<Self, String> {
        if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
            return Err("Некорректное разрешение VP9".into());
        }
        if duration_us > i32::MAX as u32 {
            return Err("Некорректная длительность VP9-кадра".into());
        }
        let buffer = OutputBuffer::default();
        let stream_io = format::context::StreamIo::from_write_with_capacity(buffer.clone(), 4096)
            .map_err(|error| error.to_string())?;
        let mut output = format::output_to_stream(stream_io, None, Some("webm"))
            .map_err(|error| error.to_string())?;
        {
            let mut stream = output
                .add_stream(None::<ffmpeg_next::Codec>)
                .map_err(|error| error.to_string())?;
            stream.set_time_base((1, 1_000_000));
            if duration_us > 0 {
                stream.set_avg_frame_rate((1_000_000, duration_us as i32));
            } else {
                tracing::debug!(
                    width,
                    height,
                    "VP9 duration unspecified; WebM uses packet timestamps"
                );
            }
            let mut parameters = stream.parameters();
            unsafe {
                let parameters = &mut *parameters.as_mut_ptr();
                parameters.codec_type = ffi::AVMediaType::AVMEDIA_TYPE_VIDEO;
                parameters.codec_id = ffi::AVCodecID::AV_CODEC_ID_VP9;
                parameters.format = ffi::AVPixelFormat::AV_PIX_FMT_YUV420P as i32;
                parameters.width = width as i32;
                parameters.height = height as i32;
            }
        }
        let mut options = Dictionary::new();
        options.set("live", "1");
        options.set("cluster_time_limit", "0");
        options.set("cluster_size_limit", "1");
        output
            .write_header_with(options)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            output,
            buffer,
            origin_us: None,
            previous_timestamp: None,
        })
    }

    /// Упаковывает один пакет; первый результат содержит WebM header.
    ///
    /// duration_us = 0 сохраняется как неизвестная длительность пакета,
    /// включая кадры, пришедшие из WebCodecs без duration.
    ///
    /// # Errors
    /// Отклоняет пустые пакеты, delta до key frame, обратное время и ошибки muxer.
    pub fn push(
        &mut self,
        bytes: &[u8],
        timestamp_us: u64,
        duration_us: u32,
        key_frame: bool,
    ) -> Result<Vec<u8>, String> {
        if bytes.is_empty()
            || bytes.len() > 8 * 1024 * 1024
            || duration_us > i32::MAX as u32
            || timestamp_us > i64::MAX as u64
        {
            return Err("Некорректный VP9-пакет WebM".into());
        }
        if self.origin_us.is_none() && !key_frame {
            return Err("Для запуска WebM нужен ключевой VP9-кадр".into());
        }
        if self
            .previous_timestamp
            .is_some_and(|previous| timestamp_us < previous)
        {
            return Err("Временные метки WebM идут назад".into());
        }
        let origin = *self.origin_us.get_or_insert(timestamp_us);
        let mut packet = Packet::copy(bytes);
        packet.set_stream(0);
        packet.set_pts(Some((timestamp_us - origin) as i64));
        packet.set_dts(packet.pts());
        packet.set_duration(i64::from(duration_us));
        if key_frame {
            packet.set_flags(packet::Flags::KEY);
        }
        let time_base = self
            .output
            .stream(0)
            .ok_or("Нет видеодорожки WebM")?
            .time_base();
        packet.rescale_ts((1, 1_000_000), time_base);
        packet
            .write(&mut self.output)
            .map_err(|error| error.to_string())?;
        unsafe {
            // NULL завершает текущий Cluster, не закрывая live Segment.
            check(ffi::av_write_frame(
                self.output.as_mut_ptr(),
                std::ptr::null_mut(),
            ))?;
            let io = (*self.output.as_mut_ptr()).pb;
            ffi::avio_flush(io);
            check((*io).error)?;
        }
        self.previous_timestamp = Some(timestamp_us);
        let mut buffer = self.buffer.0.lock().map_err(|_| "WebM buffer poisoned")?;
        Ok(std::mem::take(&mut *buffer))
    }
}
