//! Заглушка paste-вложений для неподдерживаемых клиентских платформ.

use super::super::super::pending_attachment::PendingImageAttachment;
use dioxus::prelude::*;

pub(crate) fn read_pasted_image(
    _event: ClipboardEvent,
    _on_outcome: EventHandler<Result<PendingImageAttachment, String>>,
) -> bool {
    false
}
