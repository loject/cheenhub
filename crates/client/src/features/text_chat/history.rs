//! Text chat history loading.

use std::rc::Rc;

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;

use super::messages::prepend_messages;
use super::realtime;
use super::scroll::{ScrollCommand, capture_scroll_position};

#[derive(Clone)]
pub(super) struct HistoryTarget {
    pub(super) realtime: RealtimeHandle,
    pub(super) server_id: String,
    pub(super) room_id: String,
}

#[derive(Clone, Copy)]
pub(super) struct HistoryState {
    pub(super) messages: Signal<Vec<TextChatMessage>>,
    pub(super) appearing_message_ids: Signal<Vec<String>>,
    pub(super) has_more: Signal<bool>,
    pub(super) initial_loading: Signal<bool>,
    pub(super) history_error: Signal<Option<String>>,
    pub(super) older_loading: Signal<bool>,
    pub(super) older_error: Signal<Option<String>>,
    pub(super) list_element: Signal<Option<Rc<MountedData>>>,
    pub(super) pending_scroll: Signal<Option<ScrollCommand>>,
}

pub(super) fn load_initial_history(target: HistoryTarget, mut state: HistoryState) {
    state.initial_loading.set(true);
    state.history_error.set(None);
    spawn(async move {
        match realtime::load_room_history(&target.realtime, target.server_id, target.room_id, None)
            .await
        {
            Ok(history) => {
                state.messages.set(history.messages);
                state.appearing_message_ids.set(Vec::new());
                state.has_more.set(history.has_more);
                state.pending_scroll.set(Some(ScrollCommand::Bottom));
            }
            Err(error) => state.history_error.set(Some(error.to_string())),
        }
        state.initial_loading.set(false);
    });
}

pub(super) fn load_older_history(target: HistoryTarget, mut state: HistoryState) {
    if (state.older_loading)() || !(state.has_more)() {
        return;
    }
    let Some(before_message_id) = (state.messages)().first().map(|message| message.id.clone())
    else {
        return;
    };

    state.older_loading.set(true);
    state.older_error.set(None);
    spawn(async move {
        let before_scroll = match state.list_element.cloned() {
            Some(element) => capture_scroll_position(element).await,
            None => None,
        };

        match realtime::load_room_history(
            &target.realtime,
            target.server_id,
            target.room_id,
            Some(before_message_id),
        )
        .await
        {
            Ok(history) => {
                prepend_messages(&mut state.messages, history.messages);
                state.has_more.set(history.has_more);
                if let Some((offset_y, height)) = before_scroll {
                    state
                        .pending_scroll
                        .set(Some(ScrollCommand::Preserve { offset_y, height }));
                }
            }
            Err(error) => state.older_error.set(Some(error.to_string())),
        }
        state.older_loading.set(false);
    });
}
