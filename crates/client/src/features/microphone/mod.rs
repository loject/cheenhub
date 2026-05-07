//! Microphone capture and encoding feature.

mod backend;
mod browser;
mod provider;

#[allow(unused_imports)]
pub(crate) use backend::{
    EncodedMicrophoneFrame, MicrophoneBackend, MicrophoneCodec, MicrophoneConfig, MicrophoneError,
    MicrophoneFrameCallback, MicrophoneSession, MicrophoneStatus,
};
pub(crate) use provider::{MicrophoneHandle, MicrophoneProvider};
