//! Кодирование I420 в VP9 profile 0 через ffmpeg-next с программным резервом.

use crate::{check, platform, validate_dimensions};
use ffmpeg_next::{Dictionary, Packet, codec, encoder, format, frame, picture};

/// Сжатый VP9-пакет с timestamp исходного кадра в микросекундах.
pub struct EncodedPacket {
    /// Независимая копия пакета, сохраняющаяся после следующего encode.
    pub bytes: Vec<u8>,
    /// Время кадра в микросекундах от часов источника.
    pub timestamp_us: u64,
    /// Признак независимо декодируемого ключевого кадра.
    pub key_frame: bool,
}

/// Encoder одного VP9-потока: аппаратные варианты, затем программный резерв.
pub struct Vp9Encoder {
    encoder: encoder::video::Encoder,
    software: frame::Video,
    input_len: usize,
    hardware: bool,
}

impl Vp9Encoder {
    /// Выбирает encoder для чётных размеров, FPS и битрейта в бит/с.
    ///
    /// При недоступности аппаратных вариантов открывает программный VP9.
    ///
    /// # Errors
    /// Отклоняет неверные параметры и отказ всех вариантов, включая программный.
    pub fn new(width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self, String> {
        Self::with_candidates(width, height, fps, bitrate, platform::candidates())
    }

    fn with_candidates(
        width: u32,
        height: u32,
        fps: u32,
        bitrate: u32,
        candidates: &[platform::Candidate],
    ) -> Result<Self, String> {
        validate_dimensions(width, height)?;
        if fps == 0 || fps > 240 || bitrate == 0 || bitrate > i32::MAX as u32 {
            return Err("Некорректные FPS или битрейт VP9".into());
        }
        let mut failures = Vec::new();
        for candidate in candidates {
            let attempt = || -> Result<encoder::video::Encoder, String> {
                let codec = encoder::find_by_name(candidate.name)
                    .ok_or("Encoder отсутствует в сборке FFmpeg")?;
                let mut video = codec::Context::new_with_codec(codec)
                    .encoder()
                    .video()
                    .map_err(|error| error.to_string())?;
                video.set_width(width);
                video.set_height(height);
                video.set_bit_rate(bitrate as usize);
                video.set_time_base((1, 1_000_000));
                video.set_frame_rate(Some((fps as i32, 1)));
                video.set_gop(fps * 2);
                video.set_max_b_frames(0);
                video.set_threading(codec::threading::Config::count(4));
                unsafe {
                    let context = &mut *video.as_mut_ptr();
                    context.profile = 0;
                    context.rc_max_rate = i64::from(bitrate);
                    if !candidate.is_hardware() {
                        context.rc_min_rate = i64::from(bitrate);
                    }
                    context.rc_buffer_size = bitrate as i32;
                    context.color_range = ffmpeg_next::ffi::AVColorRange::AVCOL_RANGE_MPEG;
                }
                platform::configure(&mut video, candidate)?;
                let mut options = Dictionary::new();
                if candidate.name == "vp9_qsv" {
                    options.set("async_depth", "1");
                }
                if !candidate.is_hardware() {
                    options.set("deadline", "realtime");
                    options.set("cpu-used", "7");
                    options.set("lag-in-frames", "0");
                    options.set("row-mt", "1");
                }
                video
                    .open_as_with(codec, options)
                    .map_err(|error| error.to_string())
            };
            match attempt() {
                Ok(encoder) => {
                    let mut software = frame::Video::empty();
                    software.set_format(candidate.input_format());
                    software.set_width(width);
                    software.set_height(height);
                    check(unsafe {
                        ffmpeg_next::ffi::av_frame_get_buffer(software.as_mut_ptr(), 32)
                    })?;
                    tracing::info!(
                        encoder = candidate.name,
                        hardware = candidate.is_hardware(),
                        width,
                        height,
                        fps,
                        bitrate,
                        "opened FFmpeg VP9 encoder"
                    );
                    return Ok(Self {
                        encoder,
                        software,
                        input_len: width as usize * height as usize * 3 / 2,
                        hardware: candidate.is_hardware(),
                    });
                }
                Err(error) => {
                    tracing::debug!(encoder = candidate.name, %error, "VP9 encoder candidate unavailable");
                    failures.push(format!("{}: {error}", candidate.name));
                }
            }
        }
        let detail = failures.join("; ");
        tracing::warn!(%detail, "all VP9 encoder candidates failed");
        Err(format!("Кодирование VP9 недоступно: {detail}"))
    }

    /// Завершает кодирование и возвращает все отложенные пакеты в порядке FFmpeg.
    ///
    /// Контекст освобождается до возврата результата; продолжать encode нельзя.
    ///
    /// # Errors
    /// Возвращает ошибку, если FFmpeg не завершил encoder или вернул некорректный пакет.
    pub fn finish(mut self) -> Result<Vec<EncodedPacket>, String> {
        self.encoder
            .send_eof()
            .map_err(|error| format!("Не удалось завершить VP9 encoder: {error}"))?;
        self.receive_packets(true)
    }

    /// Передаёт плотно упакованный I420 и возвращает готовые пакеты VP9.
    ///
    /// Пустой результат означает аппаратную задержку; timestamp измеряется в мкс.
    /// # Errors
    /// Отклоняет неверную длину, непредставимое время и ошибки encoder/драйвера.
    pub fn encode(
        &mut self,
        i420: &[u8],
        timestamp_us: u64,
        key_frame: bool,
    ) -> Result<Vec<EncodedPacket>, String> {
        if i420.len() != self.input_len || timestamp_us > i64::MAX as u64 {
            return Err("Некорректный I420-кадр или timestamp VP9".into());
        }
        write_i420(&mut self.software, i420)?;
        self.software.set_pts(Some(timestamp_us as i64));
        self.software.set_kind(if key_frame {
            picture::Type::I
        } else {
            picture::Type::None
        });
        let hardware = self
            .hardware
            .then(|| platform::upload(&self.encoder, &self.software))
            .transpose()?;
        self.encoder
            .send_frame(hardware.as_ref().unwrap_or(&self.software))
            .map_err(|error| error.to_string())?;
        self.receive_packets(false)
    }

    fn receive_packets(&mut self, finishing: bool) -> Result<Vec<EncodedPacket>, String> {
        let mut output = Vec::new();
        loop {
            let mut packet = Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(()) => {
                    let timestamp_us = packet
                        .pts()
                        .and_then(|pts| u64::try_from(pts).ok())
                        .ok_or("FFmpeg вернул некорректный timestamp")?;
                    output.push(EncodedPacket {
                        bytes: packet.data().ok_or("Пустой VP9-пакет")?.to_vec(),
                        timestamp_us,
                        key_frame: packet.is_key(),
                    });
                }
                Err(ffmpeg_next::Error::Other {
                    errno: ffmpeg_next::error::EAGAIN,
                }) if !finishing => break,
                Err(ffmpeg_next::Error::Eof) if finishing => break,
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(output)
    }
}

impl Drop for Vp9Encoder {
    fn drop(&mut self) {
        tracing::debug!("closing FFmpeg VP9 encoder");
    }
}

fn write_i420(frame: &mut frame::Video, i420: &[u8]) -> Result<(), String> {
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    if i420.len() != width * height * 3 / 2 {
        return Err("Некорректная длина I420-кадра".into());
    }
    if !matches!(frame.format(), format::Pixel::NV12 | format::Pixel::YUV420P) {
        return Err("Неподдерживаемый CPU-формат VP9 encoder".into());
    }
    check(unsafe { ffmpeg_next::ffi::av_frame_make_writable(frame.as_mut_ptr()) })?;
    let stride = frame.stride(0);
    for row in 0..height {
        frame.data_mut(0)[row * stride..row * stride + width]
            .copy_from_slice(&i420[row * width..(row + 1) * width]);
    }
    let u = &i420[width * height..width * height * 5 / 4];
    let v = &i420[width * height * 5 / 4..];
    if frame.format() == format::Pixel::YUV420P {
        for (plane, input) in [(1, u), (2, v)] {
            let stride = frame.stride(plane);
            for row in 0..height / 2 {
                frame.data_mut(plane)[row * stride..row * stride + width / 2]
                    .copy_from_slice(&input[row * width / 2..(row + 1) * width / 2]);
            }
        }
        return Ok(());
    }
    let stride = frame.stride(1);
    for row in 0..height / 2 {
        let target = &mut frame.data_mut(1)[row * stride..row * stride + width];
        for column in 0..width / 2 {
            target[column * 2] = u[row * width / 2 + column];
            target[column * 2 + 1] = v[row * width / 2 + column];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
