//! Контракты backend'а камеры.

use std::fmt;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;

/// Callback, вызываемый для каждого закодированного кадра камеры.
pub(crate) type CameraFrameCallback = Rc<dyn Fn(EncodedCameraFrame)>;

/// Callback, вызываемый, когда захват камеры завершается вне управления приложения.
pub(crate) type CameraEndedCallback = Rc<dyn Fn()>;

/// Кодек закодированного видео камеры.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CameraCodec {
    /// Видео VP9.
    Vp9,
}

/// Конфигурация захвата и кодирования камеры.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CameraConfig {
    /// Предпочитаемый кодек кодирования.
    pub(crate) codec: CameraCodec,
    /// Запрошенная максимальная частота кадров.
    pub(crate) frame_rate: u32,
    /// Целевой bitrate кодировщика в битах в секунду.
    pub(crate) bitrate_bps: u32,
    /// Запрошенная ширина камеры.
    pub(crate) width: u32,
    /// Запрошенная высота камеры.
    pub(crate) height: u32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            codec: CameraCodec::Vp9,
            frame_rate: 24,
            bitrate_bps: 700_000,
            width: 1280,
            height: 720,
        }
    }
}

/// Callback'и камеры, предоставленные владеющей функцией.
#[derive(Clone)]
pub(crate) struct CameraCallbacks {
    /// Callback закодированного кадра.
    pub(crate) on_frame: CameraFrameCallback,
    /// Callback завершения захвата.
    pub(crate) on_ended: CameraEndedCallback,
}

/// Один закодированный кадр камеры.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedCameraFrame {
    /// Локальный для отправителя номер кадра.
    pub(crate) sequence: u64,
    /// Временная метка кадра в микросекундах.
    pub(crate) timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub(crate) duration_us: u32,
    /// Кодек закодированного кадра.
    pub(crate) codec: CameraCodec,
    /// Может ли этот кадр открыть поток декодера.
    pub(crate) key_frame: bool,
    /// Ширина закодированного кадра.
    pub(crate) width: u32,
    /// Высота закодированного кадра.
    pub(crate) height: u32,
    /// Сырые байты закодированного кадра.
    pub(crate) bytes: Vec<u8>,
}

/// Текущее состояние камеры.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CameraStatus {
    /// Захват остановлен.
    Idle,
    /// Браузер или backend запускает разрешение и захват.
    Starting,
    /// Захват и кодирование активны.
    Live,
    /// Браузер или ОС запретили доступ к камере.
    PermissionDenied,
    /// Последняя операция камеры завершилась ошибкой.
    Error(String),
}

/// Активная сессия камеры.
pub(crate) trait CameraSession {
    /// Останавливает захват и освобождает ресурсы backend'а.
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), CameraError>>;
}

/// Backend захвата камеры.
pub(crate) trait CameraBackend {
    /// Запускает захват и вызывает `on_frame` для каждого закодированного кадра.
    fn start(
        &self,
        config: CameraConfig,
        callbacks: CameraCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn CameraSession>, CameraError>>;
}

/// Ошибка backend'а камеры.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CameraError {
    message: String,
    kind: CameraErrorKind,
}

impl CameraError {
    /// Создает ошибку камеры из пользовательского сообщения.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self::with_kind(message, CameraErrorKind::Unavailable)
    }

    /// Создает ошибку камеры с заданной категорией.
    pub(super) fn with_kind(message: impl Into<String>, kind: CameraErrorKind) -> Self {
        Self {
            message: message.into(),
            kind,
        }
    }

    /// Возвращает, запретил ли пользователь или браузер доступ к камере.
    pub(crate) fn is_permission_denied(&self) -> bool {
        self.kind == CameraErrorKind::PermissionDenied
    }

    /// Возвращает, что текущий браузер не поддерживает камеру.
    pub(crate) fn is_unsupported_browser(&self) -> bool {
        self.kind == CameraErrorKind::UnsupportedBrowser
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CameraErrorKind {
    PermissionDenied,
    UnsupportedBrowser,
    Unavailable,
}

impl fmt::Display for CameraError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CameraError {}
