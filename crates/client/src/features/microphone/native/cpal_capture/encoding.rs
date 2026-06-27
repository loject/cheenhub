//! Native-кодирование PCM микрофона в Opus.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use dioxus::prelude::{debug, spawn, warn};
use futures_channel::mpsc as local_mpsc;
use futures_util::StreamExt;
use opus::{Application, Bitrate, Channels, Encoder, Signal};

use super::super::super::backend::{
    EncodedMicrophoneFrame, MicrophoneCallbacks, MicrophoneCodec, MicrophoneConfig,
    MicrophoneError, MicrophoneLevel,
};
use super::super::super::vad::{VoiceActivityDetector, rms_level};

const OPUS_FRAME_DURATION_US: u32 = 20_000;
const MAX_OPUS_PACKET_BYTES: usize = 4_000;

pub(super) fn spawn_encoder_worker(
    config: MicrophoneConfig,
    pcm_receiver: mpsc::Receiver<Vec<f32>>,
    event_sender: local_mpsc::UnboundedSender<NativeMicrophoneEvent>,
    closed: Arc<AtomicBool>,
    bitrate_bps: Arc<AtomicU32>,
    frame_samples: usize,
) {
    thread::Builder::new()
        .name("cheenhub-microphone-encoder".to_owned())
        .spawn(move || {
            if let Err(error) = run_encoder_worker(
                config,
                pcm_receiver,
                event_sender,
                closed,
                bitrate_bps,
                frame_samples,
            ) {
                warn!(%error, "native microphone encoder worker stopped with error");
            }
        })
        .map(|_| ())
        .unwrap_or_else(|error| {
            warn!(
                error = %error,
                "failed to spawn native microphone encoder worker"
            );
        });
}

pub(super) fn spawn_event_relay(
    mut receiver: local_mpsc::UnboundedReceiver<NativeMicrophoneEvent>,
    callbacks: MicrophoneCallbacks,
) {
    spawn(async move {
        while let Some(event) = receiver.next().await {
            match event {
                NativeMicrophoneEvent::Frame(frame) => (callbacks.on_frame)(frame),
                NativeMicrophoneEvent::Level(level) => (callbacks.on_level)(level),
            }
        }
        debug!("native microphone event relay stopped");
    });
}

fn run_encoder_worker(
    config: MicrophoneConfig,
    pcm_receiver: mpsc::Receiver<Vec<f32>>,
    mut event_sender: local_mpsc::UnboundedSender<NativeMicrophoneEvent>,
    closed: Arc<AtomicBool>,
    bitrate_bps: Arc<AtomicU32>,
    frame_samples: usize,
) -> Result<(), MicrophoneError> {
    let mut encoder = create_encoder(&config)?;
    let mut detector = VoiceActivityDetector::new(config.clone());
    let mut pending = Vec::with_capacity(frame_samples * 2);
    let mut sequence = 0_u64;
    let mut captured_samples = 0_u64;
    let mut applied_bitrate = config.bitrate_bps;

    while !closed.load(Ordering::Relaxed) {
        let Ok(mut samples) = pcm_receiver.recv() else {
            break;
        };
        pending.append(&mut samples);

        while pending.len() >= frame_samples {
            let frame: Vec<f32> = pending.drain(..frame_samples).collect();
            let timestamp_us = timestamp_us(captured_samples, config.sample_rate_hz);
            captured_samples = captured_samples.saturating_add(frame_samples as u64);
            let next_bitrate = bitrate_bps.load(Ordering::Relaxed);
            if next_bitrate != applied_bitrate {
                encoder
                    .set_bitrate(Bitrate::Bits(next_bitrate.min(i32::MAX as u32) as i32))
                    .map_err(opus_error)?;
                applied_bitrate = next_bitrate;
                debug!(
                    bitrate_bps = next_bitrate,
                    "native microphone opus bitrate updated"
                );
            }

            handle_pcm_frame(
                &mut encoder,
                &mut detector,
                &mut event_sender,
                &config,
                frame,
                sequence,
                timestamp_us,
            )?;
            sequence = sequence.saturating_add(1);
        }
    }

    Ok(())
}

fn create_encoder(config: &MicrophoneConfig) -> Result<Encoder, MicrophoneError> {
    let mut encoder = Encoder::new(config.sample_rate_hz, Channels::Mono, Application::Voip)
        .map_err(opus_error)?;
    encoder
        .set_bitrate(Bitrate::Bits(config.bitrate_bps.min(i32::MAX as u32) as i32))
        .map_err(opus_error)?;
    encoder.set_signal(Signal::Voice).map_err(opus_error)?;
    encoder.set_inband_fec(true).map_err(opus_error)?;
    encoder.set_packet_loss_perc(10).map_err(opus_error)?;
    encoder.set_dtx(true).map_err(opus_error)?;
    Ok(encoder)
}

fn handle_pcm_frame(
    encoder: &mut Encoder,
    detector: &mut VoiceActivityDetector,
    event_sender: &mut local_mpsc::UnboundedSender<NativeMicrophoneEvent>,
    config: &MicrophoneConfig,
    mut frame: Vec<f32>,
    sequence: u64,
    timestamp_us: u64,
) -> Result<(), MicrophoneError> {
    apply_input_gain(&mut frame, config.input_gain);
    let rms = rms_level(&frame);
    let previous_active = detector.is_active();
    let active = detector.update(rms, OPUS_FRAME_DURATION_US);
    if active != previous_active {
        debug!(
            rms,
            active,
            threshold = detector.config().vad_threshold,
            timestamp_us,
            "native microphone voice activation changed"
        );
    }

    send_event(
        event_sender,
        NativeMicrophoneEvent::Level(MicrophoneLevel {
            rms,
            active,
            threshold: detector.config().vad_threshold,
            timestamp_us,
        }),
    )?;

    if !active {
        return Ok(());
    }

    let bytes = encoder
        .encode_vec_float(&frame, MAX_OPUS_PACKET_BYTES)
        .map_err(opus_error)?;
    send_event(
        event_sender,
        NativeMicrophoneEvent::Frame(EncodedMicrophoneFrame {
            sequence,
            timestamp_us,
            duration_us: OPUS_FRAME_DURATION_US,
            codec: MicrophoneCodec::Opus,
            sample_rate_hz: config.sample_rate_hz,
            channels: config.channels,
            bytes,
        }),
    )
}

fn send_event(
    event_sender: &mut local_mpsc::UnboundedSender<NativeMicrophoneEvent>,
    event: NativeMicrophoneEvent,
) -> Result<(), MicrophoneError> {
    event_sender
        .unbounded_send(event)
        .map_err(|_| MicrophoneError::new("Native-микрофон остановлен."))
}

pub(super) enum NativeMicrophoneEvent {
    Frame(EncodedMicrophoneFrame),
    Level(MicrophoneLevel),
}

fn apply_input_gain(samples: &mut [f32], input_gain: f32) {
    if (input_gain - 1.0).abs() < f32::EPSILON {
        return;
    }

    for sample in samples {
        *sample = (*sample * input_gain).clamp(-1.0, 1.0);
    }
}

pub(super) fn frame_samples(sample_rate_hz: u32) -> usize {
    ((u64::from(sample_rate_hz) * u64::from(OPUS_FRAME_DURATION_US)) / 1_000_000).max(1) as usize
}

fn timestamp_us(sample_index: u64, sample_rate_hz: u32) -> u64 {
    sample_index
        .saturating_mul(1_000_000)
        .checked_div(u64::from(sample_rate_hz.max(1)))
        .unwrap_or_default()
}

fn opus_error(error: impl std::fmt::Display) -> MicrophoneError {
    MicrophoneError::new(format!("Native audio backend opus вернул ошибку: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_size_matches_twenty_milliseconds() {
        assert_eq!(frame_samples(48_000), 960);
    }
}
