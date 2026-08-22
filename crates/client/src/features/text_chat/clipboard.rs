//! Платформенный контракт чтения изображения из события вставки.

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
