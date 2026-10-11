//! Локальный режим разработчика: активация, сохранение и параметры сетевых проверок.

use dioxus::prelude::*;
use dioxus_sdk_storage::{LocalStorage, StorageBacking};
use serde::{Deserialize, Serialize};

mod network_bridge;
/// Адаптер настроек разработчика к существующим сетевым контрактам.
pub(crate) use network_bridge::DeveloperNetworkBridge;

const STORAGE_KEY: &str = "cheenhub.developer_mode";
/// Максимальная добавочная задержка ping в миллисекундах.
pub(crate) const MAX_PING_DELAY_MS: u32 = 10_000;

/// Сохраненные параметры режима; выключенный режим не применяет сетевые переопределения.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct DeveloperSettings {
    /// Разрешает отображение инструментов разработчика.
    pub(crate) enabled: bool,
    /// Пропускает попытку WebTransport и выбирает WebSocket.
    pub(crate) force_websocket: bool,
    /// Добавочная задержка ответа ping в миллисекундах, ограниченная 10 секундами.
    pub(crate) ping_delay_ms: u32,
}

impl DeveloperSettings {
    /// Возвращает выбор WebSocket только для включенного режима.
    pub(crate) fn effective_force_websocket(self) -> bool {
        self.enabled && self.force_websocket
    }

    /// Возвращает ограниченную задержку; выключенный режим не задерживает ping.
    pub(crate) fn effective_ping_delay_ms(self) -> u32 {
        if self.enabled {
            self.ping_delay_ms.min(MAX_PING_DELAY_MS)
        } else {
            0
        }
    }
}

/// Реактивный контекст настроек, общий для профиля, инструментов и сетевых адаптеров.
#[derive(Clone, Copy)]
pub(crate) struct DeveloperModeHandle {
    settings: Signal<DeveloperSettings>,
}

impl DeveloperModeHandle {
    /// Возвращает текущие настройки и подписывает компонент на их изменения.
    pub(crate) fn current(self) -> DeveloperSettings {
        (self.settings)()
    }

    /// Включает режим, сохраняя ранее выбранные параметры.
    ///
    /// # Errors
    /// Возвращает сообщение, если Local Storage не сохранил параметры.
    pub(crate) fn enable(mut self) -> Result<(), String> {
        let settings = DeveloperSettings {
            enabled: true,
            ..self.current()
        };
        self.save(settings)?;
        info!("developer mode enabled locally");
        Ok(())
    }

    /// Сохраняет параметры инструментов и немедленно уведомляет потребителей.
    ///
    /// # Errors
    /// Возвращает сообщение при неуспешной проверке записи; активные настройки не изменяются.
    pub(crate) fn save(&mut self, mut settings: DeveloperSettings) -> Result<(), String> {
        settings.ping_delay_ms = settings.ping_delay_ms.min(MAX_PING_DELAY_MS);
        LocalStorage::set(STORAGE_KEY.to_owned(), &settings);
        if stored_settings() != Some(settings) {
            warn!("developer mode storage write verification failed");
            return Err("Не удалось сохранить настройки на этом устройстве.".to_owned());
        }
        self.settings.set(settings);
        info!(
            enabled = settings.enabled,
            force_websocket = settings.effective_force_websocket(),
            ping_delay_ms = settings.effective_ping_delay_ms(),
            "developer mode settings updated"
        );
        Ok(())
    }

    /// Очищает сохраненные параметры и возвращает обычное поведение клиента.
    ///
    /// # Errors
    /// Возвращает сообщение, если Local Storage не подтвердил очистку.
    pub(crate) fn reset(mut self) -> Result<(), String> {
        // Dioxus StorageBacking не поддерживает удаление ключа; null очищает сохраненное состояние.
        LocalStorage::set(STORAGE_KEY.to_owned(), &Option::<DeveloperSettings>::None);
        if LocalStorage::get::<Option<DeveloperSettings>>(&STORAGE_KEY.to_owned()) != Some(None) {
            warn!("developer mode storage reset verification failed");
            return Err("Не удалось сбросить настройки на этом устройстве.".to_owned());
        }
        self.settings.set(DeveloperSettings::default());
        info!("developer mode reset locally");
        Ok(())
    }
}

/// Создает контекст режима из Local Storage в корне аутентифицированного приложения.
pub(crate) fn use_developer_mode_provider() -> DeveloperModeHandle {
    let settings = use_signal(|| {
        let settings = stored_settings().unwrap_or_default();
        info!(enabled = settings.enabled, "loaded developer mode settings");
        settings
    });
    let handle = DeveloperModeHandle { settings };
    use_context_provider(move || handle);
    handle
}

fn stored_settings() -> Option<DeveloperSettings> {
    LocalStorage::get::<Option<DeveloperSettings>>(&STORAGE_KEY.to_owned()).flatten()
}

/// Проверяет точную пару пасхалки до формирования запроса изменения профиля.
pub(crate) fn is_activation_pair(nickname: &str, email: &str) -> bool {
    nickname == "developer_mode" && email == "developer_mode@cheenhub.ru"
}

#[cfg(test)]
mod tests;
