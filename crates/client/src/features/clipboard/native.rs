//! Выбор реализации буфера обмена для текущей платформы.

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "desktop.rs"]
mod platform;

#[cfg(any(target_arch = "wasm32", not(feature = "desktop")))]
#[path = "web.rs"]
mod platform;

/// Копирует текст в системный буфер обмена.
pub(super) async fn copy_text(text: String) -> Result<(), String> {
    platform::copy_text(text).await
}
