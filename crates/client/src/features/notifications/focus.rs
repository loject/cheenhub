//! Контекст текущего состояния фокуса окна приложения.

use dioxus::prelude::*;

/// Передаёт потомкам актуальное состояние фокуса окна приложения.
#[derive(Clone, Copy)]
pub(crate) struct ApplicationFocusContext {
    focused: Signal<bool>,
}

impl ApplicationFocusContext {
    /// Создаёт контекст из сигнала глобального провайдера уведомлений.
    pub(crate) fn new(focused: Signal<bool>) -> Self {
        Self { focused }
    }

    /// Возвращает, находится ли приложение в фокусе.
    pub(crate) fn is_focused(&self) -> bool {
        (self.focused)()
    }
}
