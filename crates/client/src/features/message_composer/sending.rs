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

/// Выделяет первую часть текста в пределах лимита Unicode-символов.
///
/// Предпочитает границы абзацев и слов; остаток сохраняет исходные символы
/// для возможной повторной отправки после ошибки.
pub(super) fn split_first_part(body: &str, max_chars: usize) -> (&str, &str) {
    let body = body.trim_start();
    let Some((limit_byte, _)) = body.char_indices().nth(max_chars.max(1)) else {
        return (body, "");
    };
    let prefix = &body[..limit_byte];
    let boundary = prefix
        .rfind("\n\n")
        .map(|index| index + 2)
        .or_else(|| prefix.rfind('\n').map(|index| index + 1))
        .or_else(|| {
            prefix
                .char_indices()
                .rev()
                .find(|(_, character)| character.is_whitespace())
                .map(|(index, character)| index + character.len_utf8())
        })
        .filter(|&index| !prefix[..index].trim().is_empty())
        .unwrap_or(limit_byte);
    (&body[..boundary], &body[boundary..])
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
        let mut remaining = body.as_str();
        let mut image_id = image_id;
        let mut sent_parts = 0_usize;
        loop {
            let (part, rest) = split_first_part(remaining, operations.max_body_chars);
            if part.trim().is_empty() && image_id.is_none() {
                break;
            }
            match operations
                .send
                .call((part.trim().to_owned(), image_id.clone()))
                .await
            {
                Ok(()) => {
                    sent_parts += 1;
                    if image_id.take().is_some() {
                        state.pending_attachment.set(None);
                    }
                    remaining = rest.trim_start();
                    state.draft.set(remaining.to_owned());
                    if remaining.is_empty() {
                        debug!(sent_parts, "message composer send completed");
                        break;
                    }
                }
                Err(error) => {
                    warn!(%error, sent_parts, remaining_chars = remaining.chars().count(), "message composer send stopped");
                    state.status.set(error);
                    break;
                }
            }
        }
        state.is_sending.set(false);
        on_complete.call(());
    });
}
