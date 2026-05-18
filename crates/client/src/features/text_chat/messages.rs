//! Text chat message list helpers.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

pub(super) fn append_message(
    messages: &mut Signal<Vec<TextChatMessage>>,
    appearing_message_ids: &mut Signal<Vec<String>>,
    message: TextChatMessage,
) -> bool {
    let mut next_messages = messages();
    if next_messages
        .iter()
        .any(|saved_message| saved_message.id == message.id)
    {
        return false;
    }
    let message_id = message.id.clone();
    next_messages.push(message);
    messages.set(next_messages);
    let mut next_appearing_message_ids = appearing_message_ids();
    next_appearing_message_ids.push(message_id);
    appearing_message_ids.set(next_appearing_message_ids);

    true
}

pub(super) fn prepend_messages(
    messages: &mut Signal<Vec<TextChatMessage>>,
    incoming: Vec<TextChatMessage>,
) {
    let saved_messages = messages();
    let mut next_messages = incoming
        .into_iter()
        .filter(|message| {
            !saved_messages
                .iter()
                .any(|saved_message| saved_message.id == message.id)
        })
        .collect::<Vec<_>>();

    next_messages.extend(saved_messages);
    messages.set(next_messages);
}

/// Removes a message from the list by id.
pub(super) fn remove_message(messages: &mut Signal<Vec<TextChatMessage>>, message_id: &str) {
    let next = messages()
        .into_iter()
        .filter(|m| m.id != message_id)
        .collect();
    messages.set(next);
}

pub(super) fn is_appearing_message(message_id: &str, appearing_message_ids: &[String]) -> bool {
    appearing_message_ids
        .iter()
        .any(|appearing_message_id| appearing_message_id == message_id)
}
