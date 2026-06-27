//! Интеграция клиента с системным треем.

mod handle;
mod native;
mod provider;
mod storage;

pub(crate) use handle::SystemTrayHandle;
pub(crate) use provider::SystemTrayProvider;

/// Загружает настройку сворачивания окна в трей при закрытии.
pub(crate) fn load_minimize_to_tray_on_close() -> bool {
    storage::load_minimize_to_tray_on_close()
}
