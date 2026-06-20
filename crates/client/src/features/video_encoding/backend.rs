//! Контракты кодирования видео.

use std::fmt;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;

/// Callback, вызываемый для каждого закодированного видео-кадра.
pub(crate) type EncodedVideoFrameCallback = Rc<dyn Fn(EncodedVideoFrame)>;

/// Кодек закодированного видео.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VideoCodec {
    /// Видео VP9.
    Vp9,
}

/// Тип реализации кодировщика видео.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VideoEncodingAcceleratorKind {
    /// Браузерный `VideoEncoder` из WebCodecs.
    WebCodecs,
    /// WASM-реализация, работающая на CPU.
    WasmCpu,
    /// Нативный платформенный кодировщик.
    Native,
}

/// Описание доступной реализации кодировщика.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VideoEncoderDescriptor {
    /// Стабильный идентификатор реализации.
    pub(crate) id: String,
    /// Человекочитаемое имя реализации.
    pub(crate) label: String,
    /// Тип реализации.
    pub(crate) kind: VideoEncodingAcceleratorKind,
    /// Кодеки, которые поддерживает реализация.
    pub(crate) codecs: Vec<VideoCodec>,
}

/// Конфигурация кодирования видео.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VideoEncoderConfig {
    /// Запрошенный кодек.
    pub(crate) codec: VideoCodec,
    /// Ширина закодированного кадра.
    pub(crate) width: u32,
    /// Высота закодированного кадра.
    pub(crate) height: u32,
    /// Целевая частота кадров.
    pub(crate) frame_rate: u32,
    /// Целевой bitrate в битах в секунду.
    pub(crate) bitrate_bps: u32,
}

impl VideoEncoderConfig {
    /// Создает конфигурацию VP9-кодирования.
    pub(crate) fn vp9(width: u32, height: u32, frame_rate: u32, bitrate_bps: u32) -> Self {
        Self {
            codec: VideoCodec::Vp9,
            width,
            height,
            frame_rate,
            bitrate_bps,
        }
    }
}

/// Один закодированный видео-кадр.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedVideoFrame {
    /// Локальный для отправителя номер кадра.
    pub(crate) sequence: u64,
    /// Временная метка кадра в микросекундах.
    pub(crate) timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub(crate) duration_us: u32,
    /// Кодек закодированного кадра.
    pub(crate) codec: VideoCodec,
    /// Может ли этот кадр открыть поток декодера.
    pub(crate) key_frame: bool,
    /// Ширина закодированного кадра.
    pub(crate) width: u32,
    /// Высота закодированного кадра.
    pub(crate) height: u32,
    /// Сырые байты закодированного кадра.
    pub(crate) bytes: Vec<u8>,
}

/// Активный кодировщик видео-кадров.
pub(crate) trait VideoFrameEncoder {
    /// Тип входного кадра, принадлежащий конкретной платформе.
    type InputFrame;

    /// Кодирует один входной кадр.
    fn encode(&self, frame: &Self::InputFrame, key_frame: bool) -> Result<(), VideoEncodingError>;

    /// Закрывает кодировщик и освобождает платформенные ресурсы.
    fn close(&self) -> Result<(), VideoEncodingError>;
}

/// Конкретная реализация кодировщика.
pub(crate) trait VideoEncodingAccelerator {
    /// Тип входного кадра, который принимает реализация.
    type InputFrame;

    /// Тип активного кодировщика.
    type Encoder: VideoFrameEncoder<InputFrame = Self::InputFrame>;

    /// Возвращает описание реализации.
    fn descriptor(&self) -> VideoEncoderDescriptor;

    /// Создает активный кодировщик для заданной конфигурации.
    fn create_encoder(
        &self,
        config: VideoEncoderConfig,
        on_frame: EncodedVideoFrameCallback,
    ) -> LocalBoxFuture<'static, Result<Self::Encoder, VideoEncodingError>>;
}

/// Менеджер выбора и создания кодировщиков для текущей платформы.
pub(crate) trait VideoEncodingManager {
    /// Тип входного кадра платформы.
    type InputFrame;

    /// Тип активного кодировщика платформы.
    type Encoder: VideoFrameEncoder<InputFrame = Self::InputFrame>;

    /// Возвращает доступные реализации для заданной конфигурации.
    fn available_accelerators(
        &self,
        config: VideoEncoderConfig,
    ) -> LocalBoxFuture<'static, Result<Vec<VideoEncoderDescriptor>, VideoEncodingError>>;

    /// Создает кодировщик выбранного типа.
    fn create_encoder(
        &self,
        kind: VideoEncodingAcceleratorKind,
        config: VideoEncoderConfig,
        on_frame: EncodedVideoFrameCallback,
    ) -> LocalBoxFuture<'static, Result<Self::Encoder, VideoEncodingError>>;
}

/// Ошибка кодирования видео.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VideoEncodingError {
    message: String,
    kind: VideoEncodingErrorKind,
}

impl VideoEncodingError {
    /// Создает ошибку недоступной операции кодирования.
    pub(crate) fn unavailable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: VideoEncodingErrorKind::Unavailable,
        }
    }

    /// Создает ошибку отсутствия поддержки на текущей платформе.
    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: VideoEncodingErrorKind::Unsupported,
        }
    }

    /// Возвращает, что кодирование не поддерживается текущей платформой.
    pub(crate) fn is_unsupported(&self) -> bool {
        self.kind == VideoEncodingErrorKind::Unsupported
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoEncodingErrorKind {
    Unsupported,
    Unavailable,
}

impl fmt::Display for VideoEncodingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for VideoEncodingError {}
