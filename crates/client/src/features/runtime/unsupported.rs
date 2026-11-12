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

/// Не блокирует поток исполнителя при ожидании дедлайна realtime-запроса.
///
/// В host-проверках без desktop/mobile runtime используется отдельный поток,
/// а остальное тестовое поведение `sleep_duration` остается неизменным.
pub(super) async fn sleep_request_timeout(duration: Duration) {
    let (sender, receiver) = futures_channel::oneshot::channel();
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        let _ = sender.send(());
    });
    let _ = receiver.await;
}
