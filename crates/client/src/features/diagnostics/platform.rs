//! Выбор платформенной реализации локальной диагностики.

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "platform/native.rs"]
mod implementation;

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub(crate) use implementation::{ExportLimits, LogEntry, LogFilter, export_logs, list_logs};

/// Настраивает постоянные события tracing на desktop.
#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub(super) fn initialize() {
    implementation::initialize();
}
