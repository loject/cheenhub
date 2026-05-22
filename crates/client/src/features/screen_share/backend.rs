//! Screen sharing backend contracts.

use std::fmt;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;

/// Callback invoked for every encoded screen frame.
pub(crate) type ScreenShareFrameCallback = Rc<dyn Fn(EncodedScreenShareFrame)>;

/// Callback invoked when the capture source ends outside the app controls.
pub(crate) type ScreenShareEndedCallback = Rc<dyn Fn()>;

/// Encoded screen sharing codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenShareCodec {
    /// VP9 video.
    Vp9,
}

/// Screen capture and encoding configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareConfig {
    /// Preferred encoded codec.
    pub(crate) codec: ScreenShareCodec,
    /// Requested maximum frame rate.
    pub(crate) frame_rate: u32,
    /// Target encoder bitrate in bits per second.
    pub(crate) bitrate_bps: u32,
    /// Fallback width used when the browser does not report capture settings.
    pub(crate) fallback_width: u32,
    /// Fallback height used when the browser does not report capture settings.
    pub(crate) fallback_height: u32,
}

impl Default for ScreenShareConfig {
    fn default() -> Self {
        Self {
            codec: ScreenShareCodec::Vp9,
            frame_rate: 30,
            bitrate_bps: 1_500_000,
            fallback_width: 1920,
            fallback_height: 1080,
        }
    }
}

/// Screen sharing callbacks supplied by the owning feature.
#[derive(Clone)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) struct ScreenShareCallbacks {
    /// Encoded frame callback.
    pub(crate) on_frame: ScreenShareFrameCallback,
    /// Capture-ended callback.
    pub(crate) on_ended: ScreenShareEndedCallback,
}

/// One encoded screen sharing frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedScreenShareFrame {
    /// Sender-local frame sequence.
    pub(crate) sequence: u64,
    /// Frame timestamp in microseconds.
    pub(crate) timestamp_us: u64,
    /// Frame duration in microseconds.
    pub(crate) duration_us: u32,
    /// Encoded codec.
    pub(crate) codec: ScreenShareCodec,
    /// Encoded frame width.
    pub(crate) width: u32,
    /// Encoded frame height.
    pub(crate) height: u32,
    /// Raw encoded frame bytes.
    pub(crate) bytes: Vec<u8>,
}

/// Current screen sharing status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScreenShareStatus {
    /// Capture is stopped.
    Idle,
    /// Browser or backend permission/capture startup is in flight.
    Starting,
    /// Capture and encoding are active.
    Live,
    /// Browser or OS denied screen capture permission.
    PermissionDenied,
    /// Last screen sharing operation failed.
    Error(String),
}

/// Active screen sharing session.
pub(crate) trait ScreenShareSession {
    /// Stops capture and releases backend resources.
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>>;
}

/// Screen capture backend.
pub(crate) trait ScreenShareBackend {
    /// Starts capture and calls `on_frame` for every encoded frame.
    fn start(
        &self,
        config: ScreenShareConfig,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>>;
}

/// Screen sharing backend error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareError {
    message: String,
    kind: ScreenShareErrorKind,
}

impl ScreenShareError {
    /// Builds a screen sharing error from a user-facing message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ScreenShareErrorKind::Unavailable,
        }
    }

    /// Builds a screen sharing permission-denied error.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn permission_denied(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ScreenShareErrorKind::PermissionDenied,
        }
    }

    /// Returns whether the user or browser denied screen capture.
    pub(crate) fn is_permission_denied(&self) -> bool {
        self.kind == ScreenShareErrorKind::PermissionDenied
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenShareErrorKind {
    PermissionDenied,
    Unavailable,
}

impl fmt::Display for ScreenShareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScreenShareError {}
