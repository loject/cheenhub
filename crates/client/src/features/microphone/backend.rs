//! Microphone backend contracts.

use std::fmt;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;

/// Callback invoked for every encoded microphone frame.
pub(crate) type MicrophoneFrameCallback = Rc<dyn Fn(EncodedMicrophoneFrame)>;

/// Encoded microphone codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MicrophoneCodec {
    /// Opus audio.
    Opus,
}

/// Microphone capture and encoding configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MicrophoneConfig {
    /// Preferred encoded codec.
    pub(crate) codec: MicrophoneCodec,
    /// Target sample rate in hertz.
    pub(crate) sample_rate_hz: u32,
    /// Target channel count.
    pub(crate) channels: u8,
    /// Target encoder bitrate in bits per second.
    pub(crate) bitrate_bps: u32,
}

impl Default for MicrophoneConfig {
    fn default() -> Self {
        Self {
            codec: MicrophoneCodec::Opus,
            sample_rate_hz: 48_000,
            channels: 1,
            bitrate_bps: 32_000,
        }
    }
}

/// One encoded microphone frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedMicrophoneFrame {
    /// Sender-local frame sequence.
    pub(crate) sequence: u64,
    /// Frame timestamp in microseconds.
    pub(crate) timestamp_us: u64,
    /// Frame duration in microseconds.
    pub(crate) duration_us: u32,
    /// Encoded codec.
    pub(crate) codec: MicrophoneCodec,
    /// Sample rate in hertz.
    pub(crate) sample_rate_hz: u32,
    /// Channel count.
    pub(crate) channels: u8,
    /// Raw encoded frame bytes.
    pub(crate) bytes: Vec<u8>,
}

/// Current microphone capture status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MicrophoneStatus {
    /// Capture is stopped.
    Idle,
    /// Browser or backend permission/capture startup is in flight.
    Starting,
    /// Capture and encoding are active.
    Live,
    /// Browser or OS denied microphone permission.
    PermissionDenied,
    /// Last microphone operation failed.
    Error(String),
}

/// Active microphone session.
pub(crate) trait MicrophoneSession {
    /// Stops capture and releases backend resources.
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), MicrophoneError>>;

    /// Updates target bitrate when the backend supports live reconfiguration.
    #[allow(dead_code)]
    fn set_bitrate_bps(
        &self,
        bitrate_bps: u32,
    ) -> LocalBoxFuture<'static, Result<(), MicrophoneError>>;
}

/// Microphone capture backend.
pub(crate) trait MicrophoneBackend {
    /// Starts capture and calls `on_frame` for every encoded frame.
    fn start(
        &self,
        config: MicrophoneConfig,
        on_frame: MicrophoneFrameCallback,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn MicrophoneSession>, MicrophoneError>>;
}

/// Microphone backend error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MicrophoneError {
    message: String,
    kind: MicrophoneErrorKind,
}

impl MicrophoneError {
    /// Builds a microphone error from a user-facing message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MicrophoneErrorKind::Unavailable,
        }
    }

    /// Builds a microphone permission-denied error.
    pub(crate) fn permission_denied(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: MicrophoneErrorKind::PermissionDenied,
        }
    }

    /// Returns whether the user or browser denied microphone access.
    pub(crate) fn is_permission_denied(&self) -> bool {
        self.kind == MicrophoneErrorKind::PermissionDenied
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicrophoneErrorKind {
    PermissionDenied,
    Unavailable,
}

impl fmt::Display for MicrophoneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MicrophoneError {}
