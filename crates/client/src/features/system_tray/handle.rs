//! Состояние пользовательских настроек системного трея.

use dioxus::prelude::*;

use super::storage;

/// Контекст управления поведением системного трея.
#[derive(Clone)]
pub(crate) struct SystemTrayHandle {
    minimize_to_tray_on_close: Signal<bool>,
}

impl SystemTrayHandle {
    /// Создает контекст управления системным треем.
    pub(crate) fn new(minimize_to_tray_on_close: Signal<bool>) -> Self {
        Self {
            minimize_to_tray_on_close,
        }
    }

    /// Возвращает текущее значение настройки сворачивания окна в трей при закрытии.
    pub(crate) fn minimize_to_tray_on_close(&self) -> bool {
        (self.minimize_to_tray_on_close)()
    }

    /// Обновляет настройку сворачивания окна в трей при закрытии.
    pub(crate) fn set_minimize_to_tray_on_close(&self, enabled: bool) {
        if *self.minimize_to_tray_on_close.peek() == enabled {
            return;
        }

        info!(enabled, "system tray minimize-on-close preference changed");
        storage::save_minimize_to_tray_on_close(enabled);
        let mut minimize_to_tray_on_close = self.minimize_to_tray_on_close;
        minimize_to_tray_on_close.set(enabled);
    }
}
