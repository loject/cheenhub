//! Контракты backend'а демонстрации экрана.

use std::fmt;
use std::rc::Rc;

use cheenhub_contracts::video_presets::{
    BASE_SCREEN_SHARE_VIDEO_PRESETS, VideoPresetId, VideoPresetSpec, VideoStreamSource,
};
use futures_util::future::LocalBoxFuture;

/// Callback, вызываемый для каждого закодированного кадра экрана.
pub(crate) type ScreenShareFrameCallback = Rc<dyn Fn(EncodedScreenShareFrame)>;

/// Callback, вызываемый, когда источник захвата завершается вне управления приложения.
pub(crate) type ScreenShareEndedCallback = Rc<dyn Fn()>;

/// Кодек закодированной демонстрации экрана.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenShareCodec {
    /// VP9 video.
    Vp9,
}

/// Конфигурация захвата и кодирования экрана.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareConfig {
    /// Preferred encoded codec.
    pub(crate) codec: ScreenShareCodec,
    /// Пресеты, разрешённые текущими возможностями пользователя.
    pub(crate) allowed_presets: Vec<VideoPresetId>,
}

impl ScreenShareConfig {
    /// Выбирает лучший разрешённый пресет для размеров источника.
    pub(crate) fn preset_for_capture(&self, width: u32, height: u32) -> VideoPresetSpec {
        self.allowed_presets
            .iter()
            .copied()
            .filter(|preset| preset.spec().source == VideoStreamSource::ScreenShare)
            .filter(|preset| {
                let spec = preset.spec();
                spec.width <= width && spec.height <= height
            })
            .max_by_key(|preset| {
                let spec = preset.spec();
                u64::from(spec.width) * u64::from(spec.height)
            })
            .or_else(|| {
                self.allowed_presets
                    .iter()
                    .copied()
                    .find(|preset| preset.spec().source == VideoStreamSource::ScreenShare)
            })
            .unwrap_or(VideoPresetId::Screen720p30)
            .spec()
    }
}

impl Default for ScreenShareConfig {
    fn default() -> Self {
        Self {
            codec: ScreenShareCodec::Vp9,
            allowed_presets: BASE_SCREEN_SHARE_VIDEO_PRESETS.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_720p_for_hd_source() {
        let spec = ScreenShareConfig::default().preset_for_capture(1366, 768);
        assert_eq!((spec.width, spec.height, spec.max_fps), (1280, 720, 30));
    }

    #[test]
    fn selects_1080p_for_full_hd_source() {
        let spec = ScreenShareConfig::default().preset_for_capture(2560, 1440);
        assert_eq!((spec.width, spec.height, spec.max_fps), (1920, 1080, 15));
    }
}

/// Callback'и демонстрации экрана, предоставленные владеющей функцией.
#[derive(Clone)]
pub(crate) struct ScreenShareCallbacks {
    /// Encoded frame callback.
    pub(crate) on_frame: ScreenShareFrameCallback,
    /// Capture-ended callback.
    pub(crate) on_ended: ScreenShareEndedCallback,
}

/// Один закодированный кадр демонстрации экрана.
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
    /// Whether this frame can start a decoder stream.
    pub(crate) key_frame: bool,
    /// Encoded frame width.
    pub(crate) width: u32,
    /// Encoded frame height.
    pub(crate) height: u32,
    /// Raw encoded frame bytes.
    pub(crate) bytes: Vec<u8>,
}

/// Текущее состояние демонстрации экрана.
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

/// Активная сессия демонстрации экрана.
pub(crate) trait ScreenShareSession {
    /// Stops capture and releases backend resources.
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>>;
}

/// Backend захвата экрана.
pub(crate) trait ScreenShareBackend {
    /// Starts capture and calls `on_frame` for every encoded frame.
    fn start(
        &self,
        config: ScreenShareConfig,
        callbacks: ScreenShareCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn ScreenShareSession>, ScreenShareError>>;
}

/// Ошибка backend'а демонстрации экрана.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareError {
    message: String,
    kind: ScreenShareErrorKind,
}

impl ScreenShareError {
    /// Builds a screen sharing error from a user-facing message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self::with_kind(message, ScreenShareErrorKind::Unavailable)
    }

    /// Создает ошибку демонстрации экрана с заданной категорией.
    pub(super) fn with_kind(message: impl Into<String>, kind: ScreenShareErrorKind) -> Self {
        Self {
            message: message.into(),
            kind,
        }
    }

    /// Returns whether the user or browser denied screen capture.
    pub(crate) fn is_permission_denied(&self) -> bool {
        self.kind == ScreenShareErrorKind::PermissionDenied
    }

    /// Возвращает, что текущий браузер не поддерживает демонстрацию экрана.
    pub(crate) fn is_unsupported_browser(&self) -> bool {
        self.kind == ScreenShareErrorKind::UnsupportedBrowser
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScreenShareErrorKind {
    PermissionDenied,
    UnsupportedBrowser,
    Unavailable,
}

impl fmt::Display for ScreenShareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScreenShareError {}
