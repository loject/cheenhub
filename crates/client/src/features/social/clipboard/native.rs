//! Выбор реализации буфера обмена для текущей платформы.

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "desktop.rs"]
mod platform;

#[cfg(any(target_arch = "wasm32", not(feature = "desktop")))]
#[path = "unsupported.rs"]
mod platform;

/// Асинхронно возвращает изображение из системного буфера в формате PNG.
pub(super) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    platform::read_image_png().await
}
