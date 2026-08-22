//! Заглушка буфера обмена для неподдерживаемых платформ.

use dioxus::prelude::*;

use crate::features::social::direct_message_pending_image::PendingDirectMessageImage;

/// На неподдерживаемой платформе paste-вложение отсутствует.
pub(super) fn read_pasted_image(
    _event: ClipboardEvent,
    _on_outcome: EventHandler<Result<PendingDirectMessageImage, String>>,
) -> bool {
    false
}

pub(super) fn supports_keydown_image_paste() -> bool {
    false
}

/// На этой платформе чтение изображений из буфера пока недоступно.
pub(super) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    Ok(None)
}
