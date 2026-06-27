//! Native-захват микрофона через `cpal`.

mod encoding;
mod samples;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream, StreamConfig, SupportedStreamConfig};
use dioxus::prelude::{debug, info, warn};
use futures_util::FutureExt;

use super::super::backend::{
    MicrophoneBackend, MicrophoneCallbacks, MicrophoneCodec, MicrophoneConfig, MicrophoneError,
    MicrophoneSession,
};
use encoding::{frame_samples, spawn_encoder_worker, spawn_event_relay};
use samples::capture_callback;

const INPUT_BACKLOG_WARN_CHUNKS: usize = 24;

/// Native backend микрофона на базе `cpal` и libopus.
#[derive(Debug, Default)]
pub(super) struct CpalMicrophoneBackend;

impl MicrophoneBackend for CpalMicrophoneBackend {
    fn start(
        &self,
        config: MicrophoneConfig,
        callbacks: MicrophoneCallbacks,
    ) -> futures_util::future::LocalBoxFuture<
        'static,
        Result<Rc<dyn MicrophoneSession>, MicrophoneError>,
    > {
        async move { start_cpal_session(config, callbacks).await }.boxed_local()
    }
}

struct CpalMicrophoneSession {
    stream: RefCell<Option<Stream>>,
    closed: Arc<AtomicBool>,
    bitrate_bps: Arc<AtomicU32>,
}

impl MicrophoneSession for CpalMicrophoneSession {
    fn stop(&self) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        self.closed.store(true, Ordering::Relaxed);
        let stream = self.stream.borrow_mut().take();
        async move {
            drop(stream);
            info!("native microphone capture stopped");
            Ok(())
        }
        .boxed_local()
    }

    fn set_bitrate_bps(
        &self,
        bitrate_bps: u32,
    ) -> futures_util::future::LocalBoxFuture<'static, Result<(), MicrophoneError>> {
        self.bitrate_bps.store(bitrate_bps, Ordering::Relaxed);
        async move {
            debug!(bitrate_bps, "native microphone bitrate update queued");
            Ok(())
        }
        .boxed_local()
    }
}

async fn start_cpal_session(
    config: MicrophoneConfig,
    callbacks: MicrophoneCallbacks,
) -> Result<Rc<dyn MicrophoneSession>, MicrophoneError> {
    validate_config(&config)?;

    let host = cpal::default_host();
    let device = input_device(&host, config.device_id.as_deref())?;
    let supported_config = select_input_config(&device, &config)?;
    let sample_format = supported_config.sample_format();
    let stream_config = supported_config.config();
    let input_channels = stream_config.channels.max(1);
    let input_sample_rate_hz = stream_config.sample_rate.0;
    let frame_samples = frame_samples(config.sample_rate_hz);

    let (pcm_sender, pcm_receiver) = mpsc::sync_channel(INPUT_BACKLOG_WARN_CHUNKS * 2);
    let (event_sender, event_receiver) = futures_channel::mpsc::unbounded();
    let closed = Arc::new(AtomicBool::new(false));
    let bitrate_bps = Arc::new(AtomicU32::new(config.bitrate_bps));

    spawn_event_relay(event_receiver, callbacks);
    spawn_encoder_worker(
        config.clone(),
        pcm_receiver,
        event_sender,
        closed.clone(),
        bitrate_bps.clone(),
        frame_samples,
    );

    let stream = build_input_stream(
        &device,
        &stream_config,
        sample_format,
        input_channels,
        pcm_sender,
        closed.clone(),
    )?;
    stream.play().map_err(cpal_error)?;

    info!(
        sample_rate_hz = input_sample_rate_hz,
        channels = input_channels,
        sample_format = %sample_format,
        frame_samples,
        "native microphone cpal capture started"
    );
    Ok(Rc::new(CpalMicrophoneSession {
        stream: RefCell::new(Some(stream)),
        closed,
        bitrate_bps,
    }))
}

fn validate_config(config: &MicrophoneConfig) -> Result<(), MicrophoneError> {
    if config.codec != MicrophoneCodec::Opus {
        return Err(MicrophoneError::new(
            "Native-микрофон поддерживает только Opus.",
        ));
    }
    if config.channels != 1 {
        return Err(MicrophoneError::new(
            "Native-микрофон поддерживает только mono.",
        ));
    }
    if config.sample_rate_hz != 48_000 {
        return Err(MicrophoneError::new(
            "Native-микрофон сейчас поддерживает только 48000 Гц без resampling.",
        ));
    }
    Ok(())
}

fn input_device(
    host: &cpal::Host,
    device_id: Option<&str>,
) -> Result<cpal::Device, MicrophoneError> {
    let Some(device_id) = device_id.filter(|device_id| !device_id.is_empty()) else {
        return host
            .default_input_device()
            .ok_or_else(|| MicrophoneError::new("Системный микрофон по умолчанию не найден."));
    };

    let mut devices = host.input_devices().map_err(cpal_error)?;
    let device =
        devices.find(|device| device.name().map(|name| name == device_id).unwrap_or(false));
    device.ok_or_else(|| {
        MicrophoneError::new("Выбранный native-микрофон недоступен. Проверьте устройство ввода.")
    })
}

fn select_input_config(
    device: &cpal::Device,
    config: &MicrophoneConfig,
) -> Result<SupportedStreamConfig, MicrophoneError> {
    let target_rate = SampleRate(config.sample_rate_hz);
    let mut supported_configs = device.supported_input_configs().map_err(cpal_error)?;
    let mut fallback = None;

    for range in supported_configs.by_ref() {
        if range.min_sample_rate() > target_rate || range.max_sample_rate() < target_rate {
            continue;
        }

        let candidate = range.with_sample_rate(target_rate);
        if candidate.channels() == u16::from(config.channels) {
            return Ok(candidate);
        }
        fallback.get_or_insert(candidate);
    }

    fallback.ok_or_else(|| {
        MicrophoneError::new(format!(
            "Native-микрофон не поддерживает {} Гц для захвата голоса.",
            config.sample_rate_hz
        ))
    })
}

fn build_input_stream(
    device: &cpal::Device,
    stream_config: &StreamConfig,
    sample_format: SampleFormat,
    input_channels: u16,
    pcm_sender: mpsc::SyncSender<Vec<f32>>,
    closed: Arc<AtomicBool>,
) -> Result<Stream, MicrophoneError> {
    let err_fn = move |error| {
        warn!(
            error = %error,
            "native microphone cpal stream failed"
        );
    };
    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            stream_config,
            capture_callback::<f32>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        SampleFormat::F64 => device.build_input_stream(
            stream_config,
            capture_callback::<f64>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        SampleFormat::I8 => device.build_input_stream(
            stream_config,
            capture_callback::<i8>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            stream_config,
            capture_callback::<i16>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        SampleFormat::I32 => device.build_input_stream(
            stream_config,
            capture_callback::<i32>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        SampleFormat::U8 => device.build_input_stream(
            stream_config,
            capture_callback::<u8>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            stream_config,
            capture_callback::<u16>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        SampleFormat::U32 => device.build_input_stream(
            stream_config,
            capture_callback::<u32>(input_channels, pcm_sender, closed),
            err_fn,
            None,
        ),
        _ => {
            return Err(MicrophoneError::new(format!(
                "Native-микрофон вернул неподдерживаемый sample format: {sample_format}."
            )));
        }
    };
    stream.map_err(cpal_error)
}

fn cpal_error(error: impl std::fmt::Display) -> MicrophoneError {
    MicrophoneError::new(format!("Native audio backend cpal вернул ошибку: {error}"))
}
