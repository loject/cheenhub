//! Общая последовательность загрузки вложения и отправки сообщения.

use dioxus::prelude::*;

use super::pending_attachment::can_send_message;
use super::{MessageComposeState, MessageOperations};

/// Снимает блокировку формы также при отмене Dioxus task.
struct SendingGuard(Signal<bool>);

impl Drop for SendingGuard {
    fn drop(&mut self) {
        if let Ok(mut sending) = self.0.try_write() {
            if *sending {
                debug!("message composer send canceled with owning view");
            }
            *sending = false;
        }
    }
}

pub(super) fn submit_message(
    mut state: MessageComposeState,
    operations: MessageOperations,
    on_complete: EventHandler<()>,
) {
    if !can_send_message(
        &(state.draft)(),
        (state.pending_attachment)().is_some(),
        (state.is_sending)() || (state.is_selecting_image)() || (state.is_reading_clipboard)(),
    ) {
        return;
    }
    let body = (state.draft)().trim().to_owned();
    let attachment = (state.pending_attachment)();
    state.status.set(String::new());
    state.is_sending.set(true);
    let guard = SendingGuard(state.is_sending);
    spawn(async move {
        let _guard = guard;
        let image_id = match attachment {
            Some(attachment) => match attachment.uploaded_id.clone() {
                Some(id) => Some(id),
                None => match operations.upload.call(attachment).await {
                    Ok(id) => {
                        debug!(attachment_id = %id, "message composer image uploaded");
                        if let Some(pending) = state.pending_attachment.write().as_mut() {
                            pending.uploaded_id = Some(id.clone());
                        }
                        Some(id)
                    }
                    Err(error) => {
                        warn!(%error, "message composer image upload failed");
                        state.status.set(error);
                        state.is_sending.set(false);
                        on_complete.call(());
                        return;
                    }
                },
            },
            None => None,
        };
        match operations.send.call((body, image_id)).await {
            Ok(()) => {
                state.draft.set(String::new());
                state.pending_attachment.set(None);
                debug!("message composer send completed");
            }
            Err(error) => {
                warn!(%error, "message composer send failed");
                state.status.set(error);
            }
        }
        state.is_sending.set(false);
        on_complete.call(());
    });
}
