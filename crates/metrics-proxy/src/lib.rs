#![warn(missing_docs)]
//! Сбор системных метрик Linux напрямую или через изолированный Docker proxy.
//!
//! Прямой сбор используется backend вне контейнера; Docker-сборщик сохраняет
//! прежний контракт proxy и не требует доступа backend к Docker socket.

mod collector;
mod docker;
mod environment;
mod native;
mod processes;
mod system;

pub use collector::MetricsCollector as DockerMetricsCollector;
pub use environment::running_in_container;
pub use native::NativeMetricsCollector;
