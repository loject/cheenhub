//! Платформенный контракт вставки изображения в общую форму сообщения.

mod platform;

use super::pending_attachment::PendingImageAttachment;
use dioxus::prelude::*;

/// Читает изображение из paste-события, не вмешиваясь в текстовую вставку.
pub(crate) fn read_pasted_image(
    event: ClipboardEvent,
    on_outcome: EventHandler<Result<PendingImageAttachment, String>>,
) -> bool {
    platform::read_pasted_image(event, on_outcome)
}

/// Возвращает, нужно ли текущей платформе читать системный буфер на keydown.
pub(crate) fn supports_keydown_image_paste() -> bool {
    platform::supports_keydown_image_paste()
}

/// Асинхронно возвращает изображение из системного буфера в формате PNG для desktop-клиента.
pub(crate) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    platform::read_image_png().await
}
