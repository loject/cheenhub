//! Платформенный контракт чтения изображения из буфера обмена.

mod native;

/// Асинхронно возвращает изображение из системного буфера в формате PNG.
pub(super) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    native::read_image_png().await
}
