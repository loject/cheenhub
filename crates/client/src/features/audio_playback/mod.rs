//! Browser audio playback feature.

mod backend;
mod native;
mod output_devices;
mod storage;
mod unsupported;
mod web;

pub(crate) use backend::{NotificationSound, PlaybackCodec, VoiceFrame};
pub(crate) use native::{AudioPlaybackHandle, AudioPlaybackProvider};
pub(crate) use output_devices::{
    AudioOutputDevice, AudioOutputDevicesResult, enumerate_audio_output_devices,
};
pub(crate) use storage::{MAX_JITTER_BUFFER_MS, MIN_JITTER_BUFFER_MS};
