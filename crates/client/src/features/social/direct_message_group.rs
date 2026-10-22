//! Группа последовательных личных сообщений одного автора.

use cheenhub_contracts::rest::DmMessageSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};
use crate::features::app::current_user::CurrentUserContext;
use crate::features::text_chat::{
    ChatMessageItem, InlineMessageEditState, estimated_group_height,
    estimated_image_preview_height, friendly_message_date, full_message_datetime,
    is_appearing_message, message_day_key, message_time,
};

use super::direct_message_image::DirectMessageImage;
use super::presentation::dm_as_text_message;

pub(super) type VirtualDirectMessageGroup = (String, Option<String>, f64, Vec<DmMessageSummary>);

/// Группирует соседние DM и рассчитывает метаданные виртуальной строки за один проход.
pub(super) fn prepare_direct_message_groups(
    messages: &[DmMessageSummary],
) -> Vec<VirtualDirectMessageGroup> {
    let mut groups = Vec::<Vec<DmMessageSummary>>::new();
    for message in messages {
        match groups.last_mut() {
            Some(group)
                if group.last().is_some_and(|last| {
                    last.sender_user_id == message.sender_user_id
                        && message_day_key(&last.created_at) == message_day_key(&message.created_at)
                }) =>
            {
                group.push(message.clone());
            }
            _ => groups.push(vec![message.clone()]),
        }
    }

    let mut previous_day_key = None;
    groups
        .into_iter()
        .filter_map(|group| {
            let first_message = group.first()?;
            let day_key = message_day_key(&first_message.created_at);
            let date_label = (previous_day_key.as_ref() != Some(&day_key))
                .then(|| friendly_message_date(&first_message.created_at));
            previous_day_key = Some(day_key);
            let estimated_height = estimated_group_height(
                group.iter().map(|message| message.body.chars().count()),
                group.iter().filter_map(|message| {
                    message
                        .image
                        .as_ref()
                        .map(|image| estimated_image_preview_height(image.width, image.height))
                }),
            );
            Some((
                first_message.id.clone(),
                date_label,
                estimated_height,
                group,
            ))
        })
        .collect()
}

/// Рендерит сообщения и изображения в исходном порядке внутри авторской группы.
#[component]
pub(super) fn DirectMessageGroup(
    messages: Vec<DmMessageSummary>,
    appearing_message_ids: Vec<String>,
    removing_message_ids: Vec<String>,
    on_edit: EventHandler<String>,
    inline_edit: Option<InlineMessageEditState>,
) -> Element {
    let Some(first_message) = messages.first().cloned() else {
        return rsx! {};
    };

    use_avatar_seed(first_message.sender_user_id.clone());
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own_group = first_message.sender_user_id == current_user.id;
    let sent_time = message_time(&first_message.created_at);
    let sent_datetime = full_message_datetime(&first_message.created_at);

    rsx! {
        div {
            class: "chat-message-group relative grid gap-3.5",
            style: "grid-template-columns: 2.5rem minmax(0, 1fr);",
            div {
                class: "chat-message-avatar-column sticky top-3 z-20 shrink-0 self-start pt-0.5",
                UserAvatar {
                    nickname: first_message.sender_nickname.clone(),
                    avatar_url: first_message.sender_avatar_url.clone(),
                    class: (if is_own_group { "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-[12px] font-bold text-zinc-100 ring-2 ring-blue-400/35" } else { "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-[12px] font-bold text-zinc-100 outline outline-1 outline-white/10" }).to_owned(),
                    avatar_seed: Some(first_message.sender_user_id.clone()),
                }
            }
            div {
                class: "chat-message-content min-w-0",
                div { class: "mb-1 flex items-center gap-2 px-2",
                    span { class: if is_own_group { "truncate text-[13.5px] font-semibold tracking-[-0.012em] text-[#8cb8fa]" } else { "truncate text-[13.5px] font-semibold tracking-[-0.012em] text-[#f0f2f6]" }, "{first_message.sender_nickname}" }
                    if is_own_group {
                        span { class: "rounded-md border border-blue-400/25 bg-blue-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-blue-300", "Вы" }
                    }
                    time {
                        class: "shrink-0 text-[11px] tabular-nums text-zinc-400/80",
                        datetime: "{first_message.created_at}",
                        title: "{sent_datetime}",
                        "{sent_time}"
                    }
                }
                div { class: "flex flex-col gap-0.5",
                    for (index, message) in messages.iter().cloned().enumerate() {
                        ChatMessageItem {
                            key: "{message.id}",
                            message: dm_as_text_message(message.clone()),
                            animate: is_appearing_message(&message.id, &appearing_message_ids),
                            removing: removing_message_ids.contains(&message.id),
                            show_hover_time: index > 0,
                            can_delete_messages: false,
                            on_delete: move |_| {},
                            on_edit: Some(on_edit),
                            inline_edit: inline_edit.clone(),
                            external_attachment: message.image.is_some(),
                            max_edit_chars: Some(4000),
                            if let Some(image) = message.image.clone() {
                                DirectMessageImage {
                                    conversation_id: message.conversation_id.clone(),
                                    author_user_id: message.sender_user_id.clone(),
                                    image,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
