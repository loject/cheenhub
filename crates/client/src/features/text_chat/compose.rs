//! Поведение набора текста в чате.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;

use super::messages::append_message;
use super::pending_attachment::PendingImageAttachment;
use super::realtime;
use super::scroll::ScrollCommand;

#[derive(Clone, Copy)]
pub(super) struct ComposeState {
    pub(super) draft: Signal<String>,
    pub(super) messages: Signal<Vec<TextChatMessage>>,
    pub(super) appearing_message_ids: Signal<Vec<String>>,
    pub(super) status: Signal<String>,
    pub(super) is_sending: Signal<bool>,
    pub(super) pending_attachment: Signal<Option<PendingImageAttachment>>,
    pub(super) pending_scroll: Signal<Option<ScrollCommand>>,
}

pub(super) fn send_current_message(
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    mut state: ComposeState,
    on_complete: EventHandler<()>,
) {
    let body = (state.draft)().trim().to_owned();
    let attachment = (state.pending_attachment)();
    if body.is_empty() && attachment.is_none() {
        return;
    }
    state.status.set(String::new());
    state.is_sending.set(true);

    spawn(async move {
        let attachment_id = match attachment {
            Some(attachment) => match attachment.uploaded_id {
                Some(attachment_id) => Some(attachment_id),
                None => match realtime::upload_chat_image(
                    &realtime,
                    server_id.clone(),
                    room_id.clone(),
                    attachment.file_name,
                    attachment.bytes,
                )
                .await
                {
                    Ok(uploaded) => {
                        info!(attachment_id = %uploaded.id, "uploaded pending text chat image");
                        if let Some(pending) = state.pending_attachment.write().as_mut() {
                            pending.uploaded_id = Some(uploaded.id.clone());
                        }
                        Some(uploaded.id)
                    }
                    Err(error) => {
                        warn!(%error, "text chat image upload failed");
                        state.status.set(error.to_string());
                        state.is_sending.set(false);
                        on_complete.call(());
                        return;
                    }
                },
            },
            None => None,
        };
        match realtime::send_text_message(&realtime, server_id, room_id, body, attachment_id).await
        {
            Ok(accepted) => {
                let message_id = accepted.message.id.clone();
                if append_message(
                    &mut state.messages,
                    &mut state.appearing_message_ids,
                    accepted.message,
                ) {
                    debug!("scrolling text chat after current user message send");
                    state.pending_scroll.set(Some(ScrollCommand::Bottom));
                }
                debug!(%message_id, "sent text chat message");
                state.draft.set(String::new());
                state.pending_attachment.set(None);
            }
            Err(error) => {
                warn!(%error, "text chat message send failed");
                state.status.set(error.to_string());
            }
        }
        state.is_sending.set(false);
        on_complete.call(());
    });
}
