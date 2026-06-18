//! Контракт платформенного отображения входящего видео участника.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use super::ParticipantVideoFrame;

/// Платформенно-независимая фабрика renderer'ов видео участника.
pub(crate) trait ParticipantVideoBackend {
    /// Создает renderer, привязанный к целевому элементу, которым владеет Dioxus.
    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
        source_label: &'static str,
    ) -> Result<Rc<dyn ParticipantVideoRenderer>, ParticipantVideoRenderError>;
}

/// Платформенный renderer одного видеопотока участника.
pub(crate) trait ParticipantVideoRenderer {
    /// Декодирует и рендерит один входящий закодированный VP9 кадр.
    fn decode(&self, frame: &ParticipantVideoFrame) -> Result<(), ParticipantVideoRenderError>;

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
