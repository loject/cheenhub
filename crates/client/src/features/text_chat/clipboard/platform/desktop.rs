//! Desktop-реализация чтения изображения из системного буфера обмена.

use super::super::super::pending_attachment::{PendingImageAttachment, pending_image_attachment};
use dioxus::prelude::*;
use image::ImageEncoder;

pub(crate) fn read_pasted_image(
    event: ClipboardEvent,
    on_outcome: EventHandler<Result<PendingImageAttachment, String>>,
) -> bool {
    match read_image_png() {
        Ok(Some(bytes)) => {
            event.prevent_default();
            info!(
                byte_size = bytes.len(),
                "read text chat image from desktop clipboard"
            );
            spawn(async move {
                on_outcome.call(pending_image_attachment(None, bytes, 10 * 1024 * 1024));
            });
            true
        }
        Ok(None) => false,
        Err(error) => {
            warn!(%error, "failed to read text chat image from desktop clipboard");
            false
        }
    }
}

fn read_image_png() -> Result<Option<Vec<u8>>, String> {
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
