//! Настольная реализация диагностики клиента.

#[path = "native/bootstrap.rs"]
mod bootstrap;
#[path = "native/logger.rs"]
mod logger;
#[path = "native/query.rs"]
mod query;
#[path = "native/storage.rs"]
mod storage;

/// Инициализирует общий подписчик событий нативного клиента.
pub(super) fn initialize() {
    bootstrap::initialize();
}
