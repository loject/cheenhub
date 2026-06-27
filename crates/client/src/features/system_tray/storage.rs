//! Постоянное хранение системных настроек клиента.

use dioxus::prelude::{info, warn};
use dioxus_sdk_storage::{LocalStorage, StorageBacking};

const MINIMIZE_TO_TRAY_ON_CLOSE_KEY: &str = "cheenhub.system_tray.minimize_to_tray_on_close";
const ENABLED_VALUE: &str = "true";
const DISABLED_VALUE: &str = "false";

/// Загружает настройку сворачивания окна в трей при закрытии.
pub(crate) fn load_minimize_to_tray_on_close() -> bool {
    let enabled = !matches!(
        get::<LocalStorage>(MINIMIZE_TO_TRAY_ON_CLOSE_KEY).as_deref(),
        Some(DISABLED_VALUE)
    );
    info!(enabled, "loaded system tray minimize-on-close preference");
    enabled
}

/// Сохраняет настройку сворачивания окна в трей при закрытии.
pub(crate) fn save_minimize_to_tray_on_close(enabled: bool) {
    let value = if enabled {
        ENABLED_VALUE
    } else {
        DISABLED_VALUE
    };
    set::<LocalStorage>(MINIMIZE_TO_TRAY_ON_CLOSE_KEY, value);
    info!(enabled, "saved system tray minimize-on-close preference");
}

fn get<S>(key: &str) -> Option<String>
where
    S: StorageBacking<Key = String>,
{
    S::get::<Option<String>>(&key.to_owned()).flatten()
}

fn set<S>(key: &str, value: &str)
where
    S: StorageBacking<Key = String>,
{
    S::set(key.to_owned(), &Some(value.to_owned()));
    if get::<S>(key).as_deref() != Some(value) {
        warn!("failed to verify system tray preference storage write");
    }
}
