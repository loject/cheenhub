//! Заглушка буфера обмена для неподдерживаемых платформ.

use dioxus::prelude::*;

use crate::features::message_composer::pending_attachment::PendingImageAttachment;

/// На неподдерживаемой платформе paste-вложение отсутствует.
pub(crate) fn read_pasted_image(
    _event: ClipboardEvent,
    _on_outcome: EventHandler<Result<PendingImageAttachment, String>>,
) -> bool {
    false
}

pub(crate) fn supports_keydown_image_paste() -> bool {
    false
}

/// На этой платформе чтение изображений из буфера пока недоступно.
pub(crate) async fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    Ok(None)
}
