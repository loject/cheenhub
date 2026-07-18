//! Состояние пользовательской настройки автоматического запуска.

use dioxus::prelude::*;

use super::native;

/// Контекст управления автоматическим запуском CheenHub.
#[derive(Clone)]
pub(crate) struct AutostartHandle {
    enabled: Signal<bool>,
    error: Signal<Option<String>>,
}

impl AutostartHandle {
    /// Создаёт контекст автоматического запуска.
    pub(crate) fn new(enabled: Signal<bool>, error: Signal<Option<String>>) -> Self {
        Self { enabled, error }
    }

    /// Сообщает, поддерживается ли автоматический запуск на текущей платформе.
    pub(crate) fn is_supported(&self) -> bool {
        native::is_supported()
    }

    /// Возвращает текущее состояние автоматического запуска.
    pub(crate) fn enabled(&self) -> bool {
        (self.enabled)()
    }

    /// Возвращает последнюю ошибку изменения автоматического запуска.
    pub(crate) fn error(&self) -> Option<String> {
        (self.error)()
    }

    /// Включает или выключает автоматический запуск.
    pub(crate) fn set_enabled(&self, enabled: bool) {
        match native::set_enabled(enabled) {
            Ok(()) => {
                let mut enabled_signal = self.enabled;
                let mut error = self.error;
                enabled_signal.set(enabled);
                error.set(None);
                info!(enabled, "updated CheenHub autostart registration");
            }
            Err(message) => {
                let mut error = self.error;
                error.set(Some(message.clone()));
                error!(enabled, error = %message, "failed to update CheenHub autostart registration");
            }
        }
    }
}
