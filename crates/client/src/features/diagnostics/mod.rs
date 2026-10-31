//! Локальная диагностика и экспорт клиентских логов.

mod platform;

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub(crate) use platform::{ExportLimits, LogEntry, LogFilter, export_logs, list_logs};

/// Включает локальную запись диагностических событий на desktop.
#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub(crate) fn initialize() {
    platform::initialize();
}

/// На web и mobile файловый подписчик не устанавливается.
#[cfg(not(all(feature = "desktop", not(target_arch = "wasm32"))))]
pub(crate) fn initialize() {}
