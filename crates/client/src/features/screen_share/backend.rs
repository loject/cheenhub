//! Контракты backend'а демонстрации экрана.

use std::fmt;
use std::rc::Rc;

use crate::features::video_encoding::VideoEncoderConfig;
use cheenhub_contracts::video_presets::{
    SCREEN_SHARE_BITRATE_LIMIT, VideoEncodingSettings, VideoStreamSource,
};
use futures_util::future::LocalBoxFuture;

/// Callback, вызываемый для каждого закодированного кадра экрана.
pub(crate) type ScreenShareFrameCallback = Rc<dyn Fn(EncodedScreenShareFrame)>;

/// Состояние реально захваченного звука демонстрации, независимо от микрофона.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ScreenShareAudioState {
    /// Пользователь не выбрал звук либо источник не является целым экраном.
    #[default]
    Off,
    /// Браузер захватывает системный звук выбранного экрана.
    Live {
        /// UUID текущего захвата аудио.
        track_id: uuid::Uuid,
        /// Браузер подтвердил исключение звука страницы CheenHub.
        own_audio_restricted: bool,
    },
    /// Захват или кодирование звука недоступны; видео может продолжать работать.
    Unavailable,
}

/// Закодированный Opus-кадр системного звука текущей демонстрации.
#[derive(Clone)]
pub(crate) struct EncodedScreenAudioFrame {
    /// UUID захвата, общий для всех его кадров.
    pub(crate) track_id: uuid::Uuid,
    /// Последовательность Opus-кадров этой дорожки.
    pub(crate) sequence: u64,
    /// Время захвата в микросекундах на шкале демонстрации.
    pub(crate) timestamp_us: u64,
    /// Длительность Opus-кадра в микросекундах.
    pub(crate) duration_us: u32,
    /// Число каналов кодировщика: один или два.
    pub(crate) channels: u8,
    /// Сырой Opus без дополнительной сетевой упаковки.
    pub(crate) bytes: Vec<u8>,
    /// Флаг закрытия исходного захвата, проверяемый перед отложенной отправкой.
    pub(super) closed: Rc<std::cell::Cell<bool>>,
}

impl EncodedScreenAudioFrame {
    /// Проверяет, что исходный захват ещё не остановлен.
    pub(crate) fn can_send(&self) -> bool {
        !self.closed.get()
    }
}

/// Callback, вызываемый, когда источник захвата завершается вне управления приложения.
pub(crate) type ScreenShareEndedCallback = Rc<dyn Fn()>;

/// Callback, вызываемый при ошибке уже запущенной сессии захвата.
pub(crate) type ScreenShareErrorCallback = Rc<dyn Fn(ScreenShareError)>;

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
    /// Максимальный тарифный битрейт в битах в секунду.
    pub(crate) max_bitrate_bps: u32,
    /// Целевой битрейт для автоматического выбора размеров источника.
    pub(crate) automatic_bitrate_bps: u32,
}

/// Точное целевое качество, выбранное до запуска захвата.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareTargetQuality {
    /// Целевая ширина кадра.
    pub(crate) width: u32,
    /// Целевая высота кадра.
    pub(crate) height: u32,
    /// Максимальная целевая частота кадров.
    pub(crate) max_fps: u32,
    /// Целевой битрейт кодировщика в битах в секунду.
    pub(crate) bitrate_bps: u32,
}

/// Источник, который должен использовать backend захвата.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScreenShareCaptureSource {
    /// Источник выбирается платформенным API, качество задаётся отдельно.
    PlatformDefault {
        /// Необязательное качество передаваемого видео.
        target: Option<ScreenShareTargetQuality>,
    },
    /// Источник выбран в собственном picker'е приложения.
    Selected {
        /// Идентификатор физического монитора.
        source_id: String,
        /// Точное качество, выбранное пользователем.
        target: ScreenShareTargetQuality,
    },
}

/// Запрос на запуск демонстрации экрана.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenShareStartRequest {
    /// Общая конфигурация screen sharing.
    pub(crate) config: ScreenShareConfig,
    /// Источник захвата.
    pub(crate) source: ScreenShareCaptureSource,
}

impl ScreenShareConfig {
    /// Проверяет качество, выбранное отправителем, и возвращает параметры кодирования.
    ///
    /// Положительные размеры и частота кадров не ограничиваются тарифным пресетом.
    /// Битрейт проверяется относительно действующего ограничения демонстрации.
    pub(crate) fn preset_for_target(
        &self,
        target: &ScreenShareTargetQuality,
    ) -> Option<VideoEncodingSettings> {
        if target.width == 0 || target.height == 0 || target.max_fps == 0 || target.bitrate_bps == 0
        {
            return None;
        }
        (target.bitrate_bps <= self.max_bitrate_bps).then_some(VideoEncodingSettings {
            source: VideoStreamSource::ScreenShare,
            width: target.width,
            height: target.height,
            max_fps: target.max_fps,
            bitrate_bps: target.bitrate_bps,
        })
    }

    /// Создаёт автоматические настройки для размеров источника.
    pub(crate) fn preset_for_capture(&self, width: u32, height: u32) -> VideoEncodingSettings {
        VideoEncodingSettings {
            source: VideoStreamSource::ScreenShare,
            width: width.max(1),
            height: height.max(1),
            max_fps: 30,
            bitrate_bps: self.automatic_bitrate_bps.min(self.max_bitrate_bps),
        }
    }
}

impl Default for ScreenShareConfig {
    fn default() -> Self {
        Self {
            codec: ScreenShareCodec::Vp9,
            max_bitrate_bps: SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps,
            automatic_bitrate_bps: SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps,
        }
    }
}

/// Вписывает источник в размеры выбранного пресета, сохраняя его пропорции.
///
/// Изображение вписывается в выбранные размеры с сохранением пропорций.
/// Нулевые размеры источника заменяются на один пиксель. Частота кадров и bitrate
/// остаются параметрами выбранного пресета.
pub(super) fn fit_capture_preset(
    width: u32,
    height: u32,
    preset: VideoEncodingSettings,
) -> VideoEncodingSettings {
    if width == 0 || height == 0 {
        return VideoEncodingSettings {
            width: 1,
            height: 1,
            ..preset
        };
    }
    let width = u64::from(width.max(1));
    let height = u64::from(height.max(1));
    let max_width = u64::from(preset.width);
    let max_height = u64::from(preset.height);
    let (encoded_width, encoded_height) = if max_width * height <= max_height * width {
        (
            max_width,
            ((height * max_width + width / 2) / width)
                .max(1)
                .min(max_height),
        )
    } else {
        (
            ((width * max_height + height / 2) / height)
                .max(1)
                .min(max_width),
            max_height,
        )
    };
    VideoEncodingSettings {
        width: encoded_width as u32,
        height: encoded_height as u32,
        ..preset
    }
}

/// Создаёт конфигурацию VP9 из проверенных параметров захвата.
pub(super) fn encoder_config(settings: VideoEncodingSettings) -> VideoEncoderConfig {
    VideoEncoderConfig::vp9(
        settings.width,
        settings.height,
        settings.max_fps,
        settings.bitrate_bps,
    )
}

#[cfg(test)]
mod tests;

/// Callback'и демонстрации экрана, предоставленные владеющей функцией.
#[derive(Clone)]
pub(crate) struct ScreenShareCallbacks {
    /// Изменение состояния системного звука, включая отсутствие аудиодорожки.
    pub(crate) on_audio_state: Rc<dyn Fn(ScreenShareAudioState)>,
    /// Encoded frame callback.
    pub(crate) on_frame: ScreenShareFrameCallback,
    /// Capture-ended callback.
    pub(crate) on_ended: ScreenShareEndedCallback,
    /// Callback, вызываемый при runtime-ошибке активной сессии.
    pub(crate) on_error: ScreenShareErrorCallback,
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
    /// Передаёт единственному отправителю ограниченную очередь системного аудио.
    ///
    /// `None` означает отсутствие аудио или уже полученную очередь. Нативный захват
    /// системного звука пока не поддерживается этим контрактом по умолчанию.
    fn take_audio_frames(
        &self,
    ) -> Option<futures_channel::mpsc::Receiver<EncodedScreenAudioFrame>> {
        None
    }
    /// Stops capture and releases backend resources.
    fn stop(&self) -> LocalBoxFuture<'static, Result<(), ScreenShareError>>;
}

/// Backend захвата экрана.
pub(crate) trait ScreenShareBackend {
    /// Позволяет выбрать качество до системного выбора источника в браузере.
    fn supports_platform_quality_picker(&self) -> bool {
        false
    }

    /// Запускает захват и вызывает `on_frame` для каждого закодированного кадра.
    fn start(
        &self,
        request: ScreenShareStartRequest,
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
