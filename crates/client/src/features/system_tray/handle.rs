//! Состояние пользовательских настроек системного трея.

use dioxus::prelude::*;

use super::{native, status::VoiceIconStatus, storage};

/// Контекст управления поведением системного трея.
#[derive(Clone)]
pub(crate) struct SystemTrayHandle {
    minimize_to_tray_on_close: Signal<bool>,
    voice_status: Signal<VoiceIconStatus>,
}

impl SystemTrayHandle {
    /// Создает контекст управления системным треем.
    pub(crate) fn new(
        minimize_to_tray_on_close: Signal<bool>,
        voice_status: Signal<VoiceIconStatus>,
    ) -> Self {
        Self {
            minimize_to_tray_on_close,
            voice_status,
        }
    }

    /// Синхронизирует единое состояние значка голосового подключения.
    pub(crate) fn set_voice_indicator(
        &self,
        connected: bool,
        microphone_muted: bool,
        output_muted: bool,
        speaking: bool,
    ) {
        let next = VoiceIconStatus::from_voice(connected, microphone_muted, output_muted, speaking);
        self.set_voice_status(next);
    }

    /// Возвращает обычный значок при уничтожении голосового провайдера.
    pub(crate) fn reset_voice_indicator(&self) {
        self.set_voice_status(VoiceIconStatus::Inactive);
    }

    fn set_voice_status(&self, next: VoiceIconStatus) {
        let mut signal = self.voice_status;
        if *signal.peek() == next {
            return;
        }
        signal.set(next);
    }

    /// Сообщает, поддерживается ли системный трей на текущей платформе.
    pub(crate) fn is_supported(&self) -> bool {
        native::is_supported()
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
