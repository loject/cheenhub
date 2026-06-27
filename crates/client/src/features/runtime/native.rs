//! Native-реализация runtime-помощников.

use std::time::Duration;

/// Асинхронно ожидает указанную продолжительность через Tokio runtime.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "desktop", feature = "mobile")
))]
pub(super) async fn sleep_duration(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Асинхронно ожидает указанную продолжительность через browser timer.
#[cfg(target_arch = "wasm32")]
pub(super) async fn sleep_duration(duration: Duration) {
    super::web::sleep_duration(duration).await;
}

/// Ожидает указанную продолжительность в host-проверках без native runtime.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(feature = "desktop", feature = "mobile"))
))]
pub(super) async fn sleep_duration(duration: Duration) {
    super::unsupported::sleep_duration(duration).await;
}

/// Запускает desktop-клиент с размером окна CheenHub.
#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub(super) fn launch_client(app: fn() -> dioxus::prelude::Element) {
    use dioxus::desktop::{
        Config, LogicalSize, WindowBuilder, WindowCloseBehaviour, icon_from_memory,
    };

    const WINDOW_WIDTH: f64 = 1280.0;
    const WINDOW_HEIGHT: f64 = 820.0;
    const WINDOW_MIN_WIDTH: f64 = 960.0;
    const WINDOW_MIN_HEIGHT: f64 = 640.0;

    let icon = icon_from_memory(include_bytes!(
        "../../../../../crates/client/public/icons/icon-512.png"
    ))
    .expect("failed to load window icon");

    let window = WindowBuilder::new()
        .with_title("CheenHub")
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT))
        .with_maximized(true);
    let close_behaviour = if crate::features::system_tray::load_minimize_to_tray_on_close() {
        WindowCloseBehaviour::WindowHides
    } else {
        WindowCloseBehaviour::WindowCloses
    };

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_icon(icon)
                .with_close_behaviour(close_behaviour)
                .with_menu(None),
        )
        .launch(app);
}

/// Запускает не-desktop клиент через стандартный launcher Dioxus.
#[cfg(not(all(feature = "desktop", not(target_arch = "wasm32"))))]
pub(super) fn launch_client(app: fn() -> dioxus::prelude::Element) {
    super::web::launch_client(app);
}
