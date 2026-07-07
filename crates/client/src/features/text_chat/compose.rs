//! Поведение набора текста в чате.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;

use super::messages::append_message;
use super::realtime;
use super::scroll::ScrollCommand;

#[derive(Clone, Copy)]
pub(super) struct ComposeState {
    pub(super) draft: Signal<String>,
    pub(super) messages: Signal<Vec<TextChatMessage>>,
    pub(super) appearing_message_ids: Signal<Vec<String>>,
    pub(super) status: Signal<String>,
    pub(super) is_sending: Signal<bool>,
    pub(super) pending_scroll: Signal<Option<ScrollCommand>>,
}

pub(super) fn send_current_message(
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    mut state: ComposeState,
) {
    let body = (state.draft)().trim().to_owned();
    if body.is_empty() {
        return;
    }
    state.draft.set(String::new());
    state.status.set(String::new());
    state.is_sending.set(true);

    spawn(async move {
        match realtime::send_text_message(&realtime, server_id, room_id, body).await {
            Ok(accepted) => {
                if append_message(
                    &mut state.messages,
                    &mut state.appearing_message_ids,
                    accepted.message,
                ) {
                    debug!("scrolling text chat after current user message send");
                    state.pending_scroll.set(Some(ScrollCommand::Bottom));
                }
            }
            Err(error) => state.status.set(error.to_string()),
        }
        state.is_sending.set(false);
    });
}
