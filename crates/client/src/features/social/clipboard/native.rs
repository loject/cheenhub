//! Выбор реализации буфера обмена для текущей платформы.

#[cfg(target_arch = "wasm32")]
#[path = "web.rs"]
mod platform;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[path = "desktop.rs"]
mod platform;

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[path = "unsupported.rs"]
mod platform;

use super::super::direct_message_pending_image::PendingDirectMessageImage;
use dioxus::prelude::*;

pub(super) fn read_pasted_image(
    event: ClipboardEvent,
    on_outcome: EventHandler<Result<PendingDirectMessageImage, String>>,
) -> bool {
    platform::read_pasted_image(event, on_outcome)
}

pub(super) fn supports_keydown_image_paste() -> bool {
    platform::supports_keydown_image_paste()
}

/// Асинхронно возвращает изображение из системного буфера в формате PNG.
pub(super) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    platform::read_image_png().await
}
