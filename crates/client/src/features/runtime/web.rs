//! Web-реализация runtime-помощников.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::time::Duration;

use gloo_timers::future::TimeoutFuture;

/// Асинхронно ожидает указанную продолжительность через browser timer.
pub(super) async fn sleep_duration(duration: Duration) {
    let milliseconds = duration.as_millis().min(u128::from(u32::MAX)) as u32;
    TimeoutFuture::new(milliseconds).await;
}

/// Запускает web/mobile клиент через стандартный launcher Dioxus.
pub(super) fn launch_client(app: fn() -> dioxus::prelude::Element) {
    dioxus::launch(app);
}
