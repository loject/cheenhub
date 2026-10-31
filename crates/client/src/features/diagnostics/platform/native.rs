//! Настольная реализация диагностики клиента.

#[path = "native/bootstrap.rs"]
mod bootstrap;
#[path = "native/logger.rs"]
mod logger;
#[path = "native/query.rs"]
mod query;
#[path = "native/storage.rs"]
mod storage;

pub(crate) use query::{ExportLimits, ExportResult, LogEntry, LogFilter, LogPage};

use std::io;
use std::path::Path;

/// Инициализирует общий подписчик событий нативного клиента.
pub(super) fn initialize() {
    bootstrap::initialize();
}

pub(crate) fn list_logs(filter: &LogFilter, offset: usize, limit: usize) -> io::Result<LogPage> {
    let directory = bootstrap::archive_directory()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Каталог логов недоступен"))?;
    query::list_logs(&directory, filter, offset, limit)
}

pub(crate) fn export_logs(
    output: &Path,
    filter: &LogFilter,
    limits: ExportLimits,
) -> io::Result<ExportResult> {
    let directory = bootstrap::archive_directory()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Каталог логов недоступен"))?;
    query::export_logs(&directory, output, filter, limits)
}
