#![warn(missing_docs)]
//! Сбор системных метрик Linux напрямую или через изолированный Docker proxy.
//!
//! Прямой сбор используется backend вне контейнера; Docker-сборщик сохраняет
//! прежний контракт proxy и не требует доступа backend к Docker socket.
//! На других ОС API доступен для сборки backend, но сбор возвращает ошибку.

#[cfg(target_os = "linux")]
mod collector;
#[cfg(target_os = "linux")]
mod docker;
mod environment;
#[cfg(target_os = "linux")]
mod native;
#[cfg(target_os = "linux")]
mod processes;
#[cfg(target_os = "linux")]
mod system;
#[cfg(not(target_os = "linux"))]
mod unsupported;

#[cfg(target_os = "linux")]
pub use collector::MetricsCollector as DockerMetricsCollector;
pub use environment::running_in_container;
#[cfg(target_os = "linux")]
pub use native::NativeMetricsCollector;
#[cfg(not(target_os = "linux"))]
pub use unsupported::{DockerMetricsCollector, NativeMetricsCollector};
