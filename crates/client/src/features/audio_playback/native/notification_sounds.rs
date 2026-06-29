//! Воспроизведение коротких WAV-уведомлений на Windows, Linux и macOS.

use dioxus::prelude::{debug, warn};

use crate::features::audio_playback::NotificationSound;

use super::mixer::queue_sender_samples;
use super::{AUDIO_SAMPLE_RATE_HZ, AudioPlaybackHandle};

const NOTIFICATION_PREROLL_MS: u32 = 40;

const CURRENT_USER_JOINED: &[u8] =
    include_bytes!("../../../../public/audio/notifications/current-user-joined.wav");
const CURRENT_USER_LEFT: &[u8] =
    include_bytes!("../../../../public/audio/notifications/current-user-left.wav");
const OTHER_USER_JOINED: &[u8] =
    include_bytes!("../../../../public/audio/notifications/other-user-joined.wav");
const OTHER_USER_LEFT: &[u8] =
    include_bytes!("../../../../public/audio/notifications/other-user-left.wav");
const CAMERA_ENABLED: &[u8] =
    include_bytes!("../../../../public/audio/notifications/camera-enabled.wav");
const CAMERA_DISABLED: &[u8] =
    include_bytes!("../../../../public/audio/notifications/camera-disabled.wav");
const SCREEN_SHARE_ENABLED: &[u8] =
    include_bytes!("../../../../public/audio/notifications/screen-share-enabled.wav");
const SCREEN_SHARE_DISABLED: &[u8] =
    include_bytes!("../../../../public/audio/notifications/screen-share-disabled.wav");
const CONNECTION_LOST: &[u8] =
    include_bytes!("../../../../public/audio/notifications/connection-lost.wav");
const CONNECTION_RESTORED: &[u8] =
    include_bytes!("../../../../public/audio/notifications/connection-restored.wav");

impl AudioPlaybackHandle {
    /// Проигрывает короткий системный звук уведомления.
    pub(crate) fn play_notification_sound(&self, sound: NotificationSound) {
        if self.is_muted() {
            return;
        }
        if let Err(error) = self.ensure_engine() {
            warn!(
                %error,
                sound = sound.event_name(),
                "failed to initialize native notification playback"
            );
            return;
        }

        let samples = match notification_samples(sound) {
            Ok(samples) => samples,
            Err(error) => {
                warn!(
                    %error,
                    sound = sound.event_name(),
                    "failed to decode native notification sound"
                );
                return;
            }
        };
        let Some(mixer) = self
            .inner
            .borrow()
            .engine
            .as_ref()
            .map(|engine| engine.mixer.clone())
        else {
            warn!(
                sound = sound.event_name(),
                "native notification playback engine disappeared before queueing"
            );
            return;
        };

        let sender_id = format!("notification:{}", sound.event_name());
        queue_sender_samples(&mixer, &sender_id, samples, 1.0, 0);
        debug!(
            sound = sound.event_name(),
            "queued native notification sound"
        );
    }
}

fn notification_samples(sound: NotificationSound) -> Result<Vec<f32>, String> {
    let bytes = match sound {
        NotificationSound::CurrentUserJoined => CURRENT_USER_JOINED,
        NotificationSound::CurrentUserLeft => CURRENT_USER_LEFT,
        NotificationSound::OtherUserJoined => OTHER_USER_JOINED,
        NotificationSound::OtherUserLeft => OTHER_USER_LEFT,
        NotificationSound::CameraEnabled => CAMERA_ENABLED,
        NotificationSound::CameraDisabled => CAMERA_DISABLED,
        NotificationSound::ScreenShareEnabled => SCREEN_SHARE_ENABLED,
        NotificationSound::ScreenShareDisabled => SCREEN_SHARE_DISABLED,
        NotificationSound::ConnectionLost => CONNECTION_LOST,
        NotificationSound::ConnectionRestored => CONNECTION_RESTORED,
    };
    let wav = decode_pcm_wav(bytes)?;
    let samples = resample_linear(wav.samples, wav.sample_rate_hz, AUDIO_SAMPLE_RATE_HZ);
    Ok(with_preroll(samples, AUDIO_SAMPLE_RATE_HZ))
}

struct WavSamples {
    sample_rate_hz: u32,
    samples: Vec<f32>,
}

fn decode_pcm_wav(bytes: &[u8]) -> Result<WavSamples, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("WAV notification asset has invalid RIFF/WAVE header.".to_owned());
    }

    let mut offset: usize = 12;
    let mut channels = None;
    let mut sample_rate_hz = None;
    let mut bits_per_sample = None;
    let mut data = None;

    while offset.saturating_add(8) <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = read_u32_le(bytes, offset + 4)? as usize;
        let start = offset + 8;
        let end = start.saturating_add(size);
        if end > bytes.len() {
            return Err("WAV notification asset has a truncated chunk.".to_owned());
        }

        match id {
            b"fmt " => {
                if size < 16 {
                    return Err("WAV notification fmt chunk is too short.".to_owned());
                }
                let format = read_u16_le(bytes, start)?;
                if format != 1 {
                    return Err("WAV notification asset must use PCM format.".to_owned());
                }
                channels = Some(read_u16_le(bytes, start + 2)?);
                sample_rate_hz = Some(read_u32_le(bytes, start + 4)?);
                bits_per_sample = Some(read_u16_le(bytes, start + 14)?);
            }
            b"data" => {
                data = Some(&bytes[start..end]);
            }
            _ => {}
        }

        offset = end + (size % 2);
    }

    let channels = channels.ok_or_else(|| "WAV notification fmt chunk is missing.".to_owned())?;
    let sample_rate_hz =
        sample_rate_hz.ok_or_else(|| "WAV notification sample rate is missing.".to_owned())?;
    let bits_per_sample =
        bits_per_sample.ok_or_else(|| "WAV notification bit depth is missing.".to_owned())?;
    let data = data.ok_or_else(|| "WAV notification data chunk is missing.".to_owned())?;
    if channels == 0 {
        return Err("WAV notification asset has zero channels.".to_owned());
    }

    decode_pcm_samples(data, channels, bits_per_sample).map(|samples| WavSamples {
        sample_rate_hz,
        samples,
    })
}

fn decode_pcm_samples(
    data: &[u8],
    channels: u16,
    bits_per_sample: u16,
) -> Result<Vec<f32>, String> {
    let channels = usize::from(channels);
    match bits_per_sample {
        8 => {
            let mut samples = Vec::with_capacity(data.len() / channels.max(1));
            for frame in data.chunks_exact(channels) {
                let sum = frame
                    .iter()
                    .map(|sample| (f32::from(*sample) - 128.0) / 128.0)
                    .sum::<f32>();
                samples.push(sum / channels as f32);
            }
            Ok(samples)
        }
        16 => {
            let bytes_per_frame = channels * 2;
            let mut samples = Vec::with_capacity(data.len() / bytes_per_frame.max(1));
            for frame in data.chunks_exact(bytes_per_frame) {
                let mut sum = 0.0;
                for channel in 0..channels {
                    let index = channel * 2;
                    let sample = i16::from_le_bytes([frame[index], frame[index + 1]]);
                    sum += f32::from(sample) / f32::from(i16::MAX);
                }
                samples.push(sum / channels as f32);
            }
            Ok(samples)
        }
        _ => Err(format!(
            "WAV notification bit depth {bits_per_sample} is not supported."
        )),
    }
}

fn resample_linear(samples: Vec<f32>, source_rate_hz: u32, target_rate_hz: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate_hz == 0 || source_rate_hz == target_rate_hz {
        return samples;
    }

    let output_len =
        ((samples.len() as u64 * u64::from(target_rate_hz)) / u64::from(source_rate_hz)) as usize;
    let output_len = output_len.max(1);
    let ratio = source_rate_hz as f64 / target_rate_hz as f64;
    let mut output = Vec::with_capacity(output_len);

    for index in 0..output_len {
        let source_position = index as f64 * ratio;
        let left = source_position.floor() as usize;
        let right = (left + 1).min(samples.len().saturating_sub(1));
        let fraction = (source_position - left as f64) as f32;
        output.push(samples[left] + (samples[right] - samples[left]) * fraction);
    }

    output
}

fn with_preroll(samples: Vec<f32>, sample_rate_hz: u32) -> Vec<f32> {
    if samples.is_empty() {
        return samples;
    }

    let preroll_samples =
        (u64::from(sample_rate_hz) * u64::from(NOTIFICATION_PREROLL_MS) / 1_000) as usize;
    let mut output = Vec::with_capacity(preroll_samples + samples.len());
    output.resize(preroll_samples, 0.0);
    output.extend(samples);
    output
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "WAV notification asset is truncated.".to_owned())?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "WAV notification asset is truncated.".to_owned())?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_all_notification_assets() {
        let sounds = [
            NotificationSound::CurrentUserJoined,
            NotificationSound::CurrentUserLeft,
            NotificationSound::OtherUserJoined,
            NotificationSound::OtherUserLeft,
            NotificationSound::CameraEnabled,
            NotificationSound::CameraDisabled,
            NotificationSound::ScreenShareEnabled,
            NotificationSound::ScreenShareDisabled,
            NotificationSound::ConnectionLost,
            NotificationSound::ConnectionRestored,
        ];

        for sound in sounds {
            let samples = notification_samples(sound).expect(sound.event_name());
            assert!(
                samples.len() > usize::try_from(AUDIO_SAMPLE_RATE_HZ / 100).unwrap(),
                "{} should produce audible PCM samples",
                sound.event_name()
            );
            assert!(
                samples.iter().any(|sample| sample.abs() > 0.01),
                "{} should not decode to silence",
                sound.event_name()
            );
        }
    }

    #[test]
    fn adds_preroll_before_notification_samples() {
        let samples = with_preroll(vec![0.5, -0.5], 1_000);

        assert_eq!(samples.len(), NOTIFICATION_PREROLL_MS as usize + 2);
        assert!(
            samples[..NOTIFICATION_PREROLL_MS as usize]
                .iter()
                .all(|sample| *sample == 0.0)
        );
        assert_eq!(samples[NOTIFICATION_PREROLL_MS as usize], 0.5);
    }
}
