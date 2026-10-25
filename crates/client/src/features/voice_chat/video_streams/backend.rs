//! Контракт платформенного отображения входящего видео участника.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;

use super::ParticipantVideoFrame;

/// Получает ширину и высоту отображаемого кадра при изменении размеров потока.
pub(crate) type ParticipantVideoDimensionsCallback = Rc<dyn Fn(u32, u32)>;

/// Платформенно-независимая фабрика renderer'ов видео участника.
pub(crate) trait ParticipantVideoBackend {
    /// Создает renderer, привязанный к целевому элементу, которым владеет Dioxus.
    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
        source_label: &'static str,
        on_dimensions: ParticipantVideoDimensionsCallback,
    ) -> Result<Rc<dyn ParticipantVideoRenderer>, ParticipantVideoRenderError>;
}

/// Платформенный renderer одного видеопотока участника.
pub(crate) trait ParticipantVideoRenderer {
    /// Возвращает URL для штатного HTML video, если выводом управляет медиаплеер.
    ///
    /// Canvas renderer возвращает `None`; URL живёт до вызова `close`.
    fn playback_url(&self) -> Option<String> {
        None
    }

    /// Асинхронно декодирует и отображает один VP9-кадр.
    fn decode(
        &self,
        frame: ParticipantVideoFrame,
    ) -> LocalBoxFuture<'_, Result<(), ParticipantVideoRenderError>>;

    /// Освобождает платформенный decoder и ресурсы рендеринга.
    fn close(&self);
}

/// Пользовательская ошибка настройки рендеринга или декодирования.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParticipantVideoRenderError {
    message: String,
}

impl ParticipantVideoRenderError {
    /// Создает ошибку рендеринга.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ParticipantVideoRenderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ParticipantVideoRenderError {}
