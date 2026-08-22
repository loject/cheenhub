//! Локальные команды изменения состояния формы сообщения.

use super::pending_attachment::PendingImageAttachment;
use super::room_compose_state::RoomComposeState;
use dioxus::prelude::*;

pub(super) fn add_pending_image(
    mut state: RoomComposeState,
    result: Result<PendingImageAttachment, String>,
) {
    if (state.is_sending)() || (state.pending_attachment)().is_some() {
        return;
    }
    match result {
        Ok(attachment) => {
            info!(
                byte_size = attachment.byte_size,
                has_file_name = attachment.file_name.is_some(),
                "added pending text chat image"
            );
            state.status.set(String::new());
            state.pending_attachment.set(Some(attachment));
        }
        Err(error) => {
            warn!(%error, "text chat image was rejected before upload");
            state.status.set(error);
        }
    }
}
