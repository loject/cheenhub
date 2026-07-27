//! Output stream на базе `cpal` для Windows, Linux и macOS.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig, SupportedStreamConfig};
use dioxus::prelude::{debug, info, warn};

use super::cpal_playback::mixer::{MixerHandle, NativeOutputMixer, new_mixer};

trait CpalOutputSample: Copy + Send + 'static {
    fn from_f32(sample: f32) -> Self;
}

impl CpalOutputSample for f32 {
    fn from_f32(sample: f32) -> Self {
        sample.clamp(-1.0, 1.0)
    }
}

impl CpalOutputSample for f64 {
    fn from_f32(sample: f32) -> Self {
        f64::from(sample.clamp(-1.0, 1.0))
    }
}

impl CpalOutputSample for i8 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i8::MAX as f32) as Self
    }
}

impl CpalOutputSample for i16 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as Self
    }
}

impl CpalOutputSample for i32 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i32::MAX as f32) as Self
    }
}

impl CpalOutputSample for u8 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * 128.0 + 128.0).clamp(0.0, u8::MAX as f32) as Self
    }
}

impl CpalOutputSample for u16 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * 32_768.0 + 32_768.0).clamp(0.0, u16::MAX as f32) as Self
    }
}

impl CpalOutputSample for u32 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * 2_147_483_648.0 + 2_147_483_648.0).clamp(0.0, u32::MAX as f32)
            as Self
    }
}

/// Активный output stream и связанный с ним микшер.
pub(super) struct NativePlaybackEngine {
    _stream: Stream,
    pub(super) mixer: MixerHandle,
    device_id: Option<String>,
}

impl NativePlaybackEngine {
    /// Возвращает идентификатор выбранного устройства вывода.
    pub(super) fn device_id(&self) -> Option<String> {
        self.device_id.clone()
    }
}

impl Drop for NativePlaybackEngine {
    fn drop(&mut self) {
        debug!(
            selected_device = self.device_id.as_deref().unwrap_or(""),
            "native audio playback output stream dropped"
        );
    }
}

/// Создает и запускает output stream для выбранного устройства.
pub(super) fn create_engine(
    device_id: Option<String>,
    output_gain: f32,
    sample_rate_hz: u32,
) -> Result<NativePlaybackEngine, String> {
    let host = cpal::default_host();
    let device = output_device(&host, device_id.as_deref())?;
    let supported_config = select_output_config(&device, sample_rate_hz)?;
    let sample_format = supported_config.sample_format();
    let stream_config = supported_config.config();
    let channels = stream_config.channels.max(1);
    let actual_sample_rate_hz = stream_config.sample_rate.0;
    let mixer = new_mixer(output_gain);
    let stream = build_output_stream(
        &device,
        &stream_config,
        sample_format,
        channels,
        sample_rate_hz,
        actual_sample_rate_hz,
        mixer.clone(),
    )?;
    stream.play().map_err(cpal_error)?;

    info!(
        sample_rate_hz = actual_sample_rate_hz,
        channels,
        sample_format = %sample_format,
        selected_device = device_id.as_deref().unwrap_or(""),
        "cpal audio playback started"
    );
    Ok(NativePlaybackEngine {
        _stream: stream,
        mixer,
        device_id,
    })
}

fn output_device(host: &cpal::Host, device_id: Option<&str>) -> Result<cpal::Device, String> {
    let Some(device_id) = device_id.filter(|device_id| !device_id.is_empty()) else {
        return host.default_output_device().ok_or_else(|| {
            "Системное устройство вывода аудио по умолчанию не найдено.".to_owned()
        });
    };

    let mut devices = host.output_devices().map_err(cpal_error)?;
    if let Some(device) =
        devices.find(|device| device.name().map(|name| name == device_id).unwrap_or(false))
    {
        return Ok(device);
    }

    warn!(
        selected_device = device_id,
        "selected cpal audio output device is unavailable; falling back to system default"
    );
    host.default_output_device().ok_or_else(|| {
        "Выбранное устройство вывода недоступно, а системное устройство по умолчанию не найдено."
            .to_owned()
    })
}

fn select_output_config(
    device: &cpal::Device,
    sample_rate_hz: u32,
) -> Result<SupportedStreamConfig, String> {
    let mut supported_configs = device.supported_output_configs().map_err(cpal_error)?;
    let mut fallback = None;

    for range in supported_configs.by_ref() {
        if range.min_sample_rate().0 > sample_rate_hz || range.max_sample_rate().0 < sample_rate_hz
        {
            continue;
        }

        let candidate = range.with_sample_rate(cpal::SampleRate(sample_rate_hz));
        if candidate.channels() >= 2 {
            return Ok(candidate);
        }
        fallback.get_or_insert(candidate);
    }

    if let Some(fallback) = fallback {
        return Ok(fallback);
    }

    let default_config = device.default_output_config().map_err(cpal_error)?;
    warn!(
        requested_sample_rate_hz = sample_rate_hz,
        fallback_sample_rate_hz = default_config.sample_rate().0,
        "native output device does not support requested sample rate; using default cpal config with resampling"
    );
    Ok(default_config)
}

fn build_output_stream(
    device: &cpal::Device,
    stream_config: &StreamConfig,
    sample_format: SampleFormat,
    channels: u16,
    source_sample_rate_hz: u32,
    output_sample_rate_hz: u32,
    mixer: MixerHandle,
) -> Result<Stream, String> {
    let err_fn = move |error| {
        warn!(
            error = %error,
            "native audio cpal output stream failed"
        );
    };
    let stream = match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            stream_config,
            output_callback::<f32>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        SampleFormat::F64 => device.build_output_stream(
            stream_config,
            output_callback::<f64>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        SampleFormat::I8 => device.build_output_stream(
            stream_config,
            output_callback::<i8>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_output_stream(
            stream_config,
            output_callback::<i16>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        SampleFormat::I32 => device.build_output_stream(
            stream_config,
            output_callback::<i32>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        SampleFormat::U8 => device.build_output_stream(
            stream_config,
            output_callback::<u8>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            stream_config,
            output_callback::<u16>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        SampleFormat::U32 => device.build_output_stream(
            stream_config,
            output_callback::<u32>(
                channels,
                source_sample_rate_hz,
                output_sample_rate_hz,
                mixer,
            ),
            err_fn,
            None,
        ),
        _ => {
            return Err(format!(
                "Native-устройство вывода вернуло неподдерживаемый sample format: {sample_format}."
            ));
        }
    };
    stream.map_err(cpal_error)
}

fn output_callback<T>(
    channels: u16,
    source_sample_rate_hz: u32,
    output_sample_rate_hz: u32,
    mixer: MixerHandle,
) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static
where
    T: CpalOutputSample,
{
    let channels = usize::from(channels.max(1));
    let mut output = NativeOutputMixer::new(source_sample_rate_hz, output_sample_rate_hz, mixer);
    move |data, _info| {
        let frame_count = data.len().div_ceil(channels);
        output.render_frames(frame_count, |frame_index, sample| {
            let start = frame_index * channels;
            let end = (start + channels).min(data.len());
            for output_sample in &mut data[start..end] {
                *output_sample = T::from_f32(sample);
            }
        });
    }
}

fn cpal_error(error: impl std::fmt::Display) -> String {
    format!("cpal backend для Windows/Linux/macOS вернул ошибку: {error}")
}

#[cfg(test)]
mod tests {
    use super::CpalOutputSample;

    #[test]
    fn converts_f32_without_overflow() {
        assert_eq!(f32::from_f32(2.0), 1.0);
        assert_eq!(f32::from_f32(-2.0), -1.0);
    }

    #[test]
    fn converts_unsigned_midpoint_to_silence() {
        assert_eq!(u8::from_f32(0.0), 128);
        assert_eq!(u16::from_f32(0.0), 32_768);
    }
}
