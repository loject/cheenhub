//! Browser audio playback feature.

mod browser_bindings;
mod browser_helpers;
mod jitter_buffer;
mod jitter_runtime;
mod output_devices;
mod playback_pipeline;
mod provider;
mod storage;

pub(crate) use output_devices::{
    AudioOutputDevice, AudioOutputDevicesResult, enumerate_audio_output_devices,
};
pub(crate) use provider::{AudioPlaybackHandle, AudioPlaybackProvider, PlaybackCodec, VoiceFrame};
