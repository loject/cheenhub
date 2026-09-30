//! Desktop-реализация чтения изображения из системного буфера обмена.

use dioxus::prelude::*;
use image::ImageEncoder;

use crate::features::message_composer::pending_attachment::PendingImageAttachment;

/// Desktop-клиент читает изображение по сочетанию клавиш, а не из browser event.
pub(crate) fn read_pasted_image(
    _event: ClipboardEvent,
    _on_outcome: EventHandler<Result<PendingImageAttachment, String>>,
) -> bool {
    false
}

pub(crate) fn supports_keydown_image_paste() -> bool {
    true
}

/// Асинхронно возвращает изображение из системного буфера в формате PNG.
pub(crate) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    tokio::task::spawn_blocking(read_image_png_blocking)
        .await
        .map_err(|error| {
            format!("Не удалось завершить чтение изображения из буфера обмена: {error}")
        })?
}

fn read_image_png_blocking() -> Result<Option<Vec<u8>>, String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|_| "Не удалось открыть системный буфер обмена.".to_owned())?;
    let image = match clipboard.get_image() {
        Ok(image) => image,
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(_) => return Err("Не удалось прочитать изображение из буфера обмена.".to_owned()),
    };
    let width = u32::try_from(image.width)
        .map_err(|_| "Изображение из буфера имеет неверный размер.".to_owned())?;
    let height = u32::try_from(image.height)
        .map_err(|_| "Изображение из буфера имеет неверный размер.".to_owned())?;
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&image.bytes, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|_| "Не удалось подготовить изображение из буфера обмена.".to_owned())?;
    Ok(Some(png))
}
