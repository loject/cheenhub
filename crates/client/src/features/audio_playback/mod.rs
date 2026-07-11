//! Browser audio playback feature.

mod backend;
mod native;
mod output_devices;
mod storage;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(
        feature = "android",
        feature = "windows",
        feature = "linux",
        feature = "macos"
    ))
))]
mod unsupported;
mod web;

pub(crate) use backend::{NotificationSound, PlaybackCodec, VoiceFrame};
pub(crate) use native::{AudioPlaybackHandle, AudioPlaybackProvider};
pub(crate) use output_devices::{
    AudioOutputDevice, AudioOutputDevicesResult, enumerate_audio_output_devices,
};
pub(crate) use storage::{MAX_JITTER_BUFFER_MS, MIN_JITTER_BUFFER_MS};
