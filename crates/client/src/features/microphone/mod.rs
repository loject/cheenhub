//! Microphone capture and encoding feature.

mod backend;
mod browser;
mod browser_bindings;
mod provider;
mod vad;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedMicrophoneFrame, MicrophoneActivationMode, MicrophoneBackend, MicrophoneCallbacks,
    MicrophoneCodec, MicrophoneConfig, MicrophoneError, MicrophoneFrameCallback, MicrophoneLevel,
    MicrophoneLevelCallback, MicrophoneSession, MicrophoneStatus,
};
pub(crate) use provider::{MicrophoneHandle, MicrophoneProvider};
