//! Платформенный контракт вставки изображения в форму личного сообщения.

mod native;

use super::direct_message_pending_image::PendingDirectMessageImage;
use dioxus::prelude::*;

/// Читает изображение из paste-события, не вмешиваясь в текстовую вставку.
pub(super) fn read_pasted_image(
    event: ClipboardEvent,
    on_outcome: EventHandler<Result<PendingDirectMessageImage, String>>,
) -> bool {
    native::read_pasted_image(event, on_outcome)
}

/// Возвращает, нужно ли текущей платформе читать системный буфер на keydown.
pub(super) fn supports_keydown_image_paste() -> bool {
    native::supports_keydown_image_paste()
}

/// Асинхронно возвращает изображение из системного буфера в формате PNG для desktop-клиента.
pub(super) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    native::read_image_png().await
}
