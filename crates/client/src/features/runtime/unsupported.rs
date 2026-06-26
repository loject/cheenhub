//! Запасная runtime-реализация для host-проверок без native feature.

#![cfg_attr(
    not(all(
        not(target_arch = "wasm32"),
        not(any(feature = "desktop", feature = "mobile"))
    )),
    allow(dead_code, unused_imports)
)]

use std::time::Duration;

/// Ожидает указанную продолжительность без платформенного async runtime.
pub(super) async fn sleep_duration(duration: Duration) {
    std::thread::sleep(duration);
}
