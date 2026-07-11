//! Платформенные runtime-помощники клиента.

pub(crate) mod android;
mod native;
mod unsupported;
mod web;

use std::time::Duration;

/// Асинхронно ожидает указанное количество миллисекунд на текущей платформе.
pub(crate) async fn sleep_ms(milliseconds: u32) {
    native::sleep_duration(Duration::from_millis(u64::from(milliseconds))).await;
}

/// Асинхронно ожидает указанную продолжительность на текущей платформе.
pub(crate) async fn sleep_duration(duration: Duration) {
    native::sleep_duration(duration).await;
}

/// Запускает клиентское приложение с настройками текущей платформы.
pub(crate) fn launch_client(app: fn() -> dioxus::prelude::Element) {
    native::launch_client(app);
}
