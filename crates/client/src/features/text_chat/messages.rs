//! Вспомогательные функции списка сообщений текстового чата.

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

/// Удаляет сообщение из списка по идентификатору.
pub(super) fn remove_message(messages: &mut Signal<Vec<TextChatMessage>>, message_id: &str) {
    let next = messages()
        .into_iter()
        .filter(|m| m.id != message_id)
        .collect();
    messages.set(next);
}

/// Обновляет существующее сообщение, сохраняя более позднюю редакцию при перестановке событий.
pub(super) fn apply_message_edit(
    messages: &mut [TextChatMessage],
    edited: TextChatMessage,
) -> bool {
    let Some(current) = messages.iter_mut().find(|current| {
        current.id == edited.id
            && current.server_id == edited.server_id
            && current.room_id == edited.room_id
    }) else {
        return false;
    };
    if edited.edited_at.is_none() {
        return false;
    }
    if let (Some(current_time), Some(next_time)) = (&current.edited_at, &edited.edited_at) {
        let current_parsed = chrono::DateTime::parse_from_rfc3339(current_time);
        let next_parsed = chrono::DateTime::parse_from_rfc3339(next_time);
        if let (Ok(current_parsed), Ok(next_parsed)) = (current_parsed, next_parsed) {
            if current_parsed > next_parsed {
                return false;
            }
        } else if current_time > next_time {
            return false;
        }
    }
    if current == &edited {
        return false;
    }
    *current = edited;
    true
}

pub(crate) fn is_appearing_message(message_id: &str, appearing_message_ids: &[String]) -> bool {
    appearing_message_ids
        .iter()
        .any(|appearing_message_id| appearing_message_id == message_id)
}

/// Группирует соседние сообщения одного автора без изменения порядка сообщений.
pub(crate) fn group_consecutive_messages(
    messages: &[TextChatMessage],
) -> Vec<Vec<TextChatMessage>> {
    let mut groups = Vec::<Vec<TextChatMessage>>::new();

    for message in messages {
        match groups.last_mut() {
            Some(group)
                if group.last().is_some_and(|last| {
                    last.author_user_id == message.author_user_id
                        && super::message_date::message_day_key(&last.created_at)
                            == super::message_date::message_day_key(&message.created_at)
                }) =>
            {
                group.push(message.clone());
            }
            _ => groups.push(vec![message.clone()]),
        }
    }

    groups
}

#[cfg(test)]
mod tests;
