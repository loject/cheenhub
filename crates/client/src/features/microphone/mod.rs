//! Microphone capture and encoding feature.

mod backend;
mod browser;
mod browser_bindings;
mod browser_capture;
mod browser_encoding;
mod browser_errors;
mod browser_worklet;
mod input_devices;
mod provider;
mod provider_context;
mod provider_runtime;
mod storage;
mod vad;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedMicrophoneFrame, MicrophoneActivationMode, MicrophoneBackend, MicrophoneCallbacks,
    MicrophoneCodec, MicrophoneConfig, MicrophoneError, MicrophoneFrameCallback, MicrophoneLevel,
    MicrophoneLevelCallback, MicrophoneSession, MicrophoneStatus,
};
pub(crate) use input_devices::{
    AudioInputDevice, AudioInputDevicesResult, enumerate_audio_input_devices,
    request_microphone_permission,
};
pub(crate) use provider::MicrophoneHandle;
pub(crate) use provider_context::MicrophoneProvider;
