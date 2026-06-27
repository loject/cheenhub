//! Преобразование native audio samples в mono PCM.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use dioxus::prelude::warn;

pub(super) fn capture_callback<T>(
    channels: u16,
    sender: mpsc::SyncSender<Vec<f32>>,
    closed: Arc<AtomicBool>,
) -> impl FnMut(&[T], &cpal::InputCallbackInfo) + Send + 'static
where
    T: CpalInputSample,
{
    let channels = usize::from(channels.max(1));
    let mut backlog_warning_emitted = false;
    move |data, _info| {
        if closed.load(Ordering::Relaxed) {
            return;
        }

        let samples = downmix_to_mono(data, channels);
        if samples.is_empty() {
            return;
        }

        match sender.try_send(samples) {
            Ok(()) => {
                backlog_warning_emitted = false;
            }
            Err(mpsc::TrySendError::Full(_)) => {
                if !backlog_warning_emitted {
                    backlog_warning_emitted = true;
                    warn!("native microphone input queue is backing up");
                }
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {}
        }
    }
}

pub(super) trait CpalInputSample: Copy + Send + 'static {
    fn to_f32(self) -> f32;
}

impl CpalInputSample for f32 {
    fn to_f32(self) -> f32 {
        self.clamp(-1.0, 1.0)
    }
}

impl CpalInputSample for f64 {
    fn to_f32(self) -> f32 {
        (self as f32).clamp(-1.0, 1.0)
    }
}

impl CpalInputSample for i8 {
    fn to_f32(self) -> f32 {
        self as f32 / i8::MAX as f32
    }
}

impl CpalInputSample for i16 {
    fn to_f32(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
}

impl CpalInputSample for i32 {
    fn to_f32(self) -> f32 {
        self as f32 / i32::MAX as f32
    }
}

impl CpalInputSample for u8 {
    fn to_f32(self) -> f32 {
        (self as f32 - 128.0) / 128.0
    }
}

impl CpalInputSample for u16 {
    fn to_f32(self) -> f32 {
        (self as f32 - 32_768.0) / 32_768.0
    }
}

impl CpalInputSample for u32 {
    fn to_f32(self) -> f32 {
        (self as f32 - 2_147_483_648.0) / 2_147_483_648.0
    }
}

fn downmix_to_mono<T: CpalInputSample>(data: &[T], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.iter().map(|sample| sample.to_f32()).collect();
    }

    data.chunks_exact(channels)
        .map(|frame| frame.iter().map(|sample| sample.to_f32()).sum::<f32>() / channels as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_interleaved_channels() {
        let samples = downmix_to_mono(&[1.0_f32, -1.0, 0.25, 0.75], 2);

        assert_eq!(samples, vec![0.0, 0.5]);
    }
}
