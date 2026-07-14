//! Microphone capture and encoding feature.

mod backend;
mod browser;
mod browser_bindings;
mod browser_capture;
mod browser_diagnostics;
mod browser_encoding;
mod browser_errors;
mod browser_worker;
mod browser_worklet;
mod core;
mod input_devices;
mod native;
mod provider;
mod provider_context;
mod provider_preferences;
mod provider_runtime;
mod storage;
#[cfg(not(any(
    feature = "web",
    target_os = "android",
    feature = "windows",
    feature = "linux",
    feature = "macos"
)))]
mod unsupported;
mod vad;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedMicrophoneFrame, MicrophoneActivationMode, MicrophoneBackend, MicrophoneCallbacks,
    MicrophoneCodec, MicrophoneConfig, MicrophoneError, MicrophoneErrorCallback,
    MicrophoneFrameCallback, MicrophoneLevel, MicrophoneLevelCallback, MicrophoneSession,
    MicrophoneStatus, MicrophoneUplinkConfig,
};
pub(crate) use input_devices::{
    AudioInputDevice, AudioInputDevicesResult, enumerate_audio_input_devices,
    request_microphone_permission,
};
pub(crate) use provider::MicrophoneHandle;
pub(crate) use provider_context::MicrophoneProvider;
