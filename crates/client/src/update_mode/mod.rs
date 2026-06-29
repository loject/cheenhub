//! Режим запуска основного приложения как установщика обновления.

mod native;

/// Запускает updater-режим, если он запрошен аргументами командной строки.
pub(crate) fn run_if_requested() -> bool {
    native::run_if_requested()
}
