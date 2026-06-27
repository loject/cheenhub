//! Native output stream на базе `cpal`.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig, SupportedStreamConfig};
use dioxus::prelude::{info, warn};

use super::mixer::{MixerHandle, new_mixer, output_callback};

/// Активный native output stream и связанный с ним микшер.
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
        mixer.clone(),
    )?;
    stream.play().map_err(cpal_error)?;

    info!(
        sample_rate_hz = actual_sample_rate_hz,
        channels,
        sample_format = %sample_format,
        selected_device = device_id.as_deref().unwrap_or(""),
        "native audio cpal playback started"
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
    let device =
        devices.find(|device| device.name().map(|name| name == device_id).unwrap_or(false));
    device.ok_or_else(|| {
        "Выбранное native-устройство вывода недоступно. Проверьте устройство вывода.".to_owned()
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

    fallback.ok_or_else(|| {
        format!("Native-устройство вывода не поддерживает {sample_rate_hz} Гц без resampling.")
    })
}

fn build_output_stream(
    device: &cpal::Device,
    stream_config: &StreamConfig,
    sample_format: SampleFormat,
    channels: u16,
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
            output_callback::<f32>(channels, mixer),
            err_fn,
            None,
        ),
        SampleFormat::F64 => device.build_output_stream(
            stream_config,
            output_callback::<f64>(channels, mixer),
            err_fn,
            None,
        ),
        SampleFormat::I8 => device.build_output_stream(
            stream_config,
            output_callback::<i8>(channels, mixer),
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_output_stream(
            stream_config,
            output_callback::<i16>(channels, mixer),
            err_fn,
            None,
        ),
        SampleFormat::I32 => device.build_output_stream(
            stream_config,
            output_callback::<i32>(channels, mixer),
            err_fn,
            None,
        ),
        SampleFormat::U8 => device.build_output_stream(
            stream_config,
            output_callback::<u8>(channels, mixer),
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            stream_config,
            output_callback::<u16>(channels, mixer),
            err_fn,
            None,
        ),
        SampleFormat::U32 => device.build_output_stream(
            stream_config,
            output_callback::<u32>(channels, mixer),
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

fn cpal_error(error: impl std::fmt::Display) -> String {
    format!("Native audio backend cpal вернул ошибку: {error}")
}
