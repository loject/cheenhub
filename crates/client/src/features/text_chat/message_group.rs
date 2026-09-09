//! Группа последовательных сообщений одного автора в текстовом чате.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};
use crate::features::app::current_user::CurrentUserContext;

use super::message_date::full_message_datetime;
use super::message_item::ChatMessageItem;
use super::message_item::message_time;
use super::messages::is_appearing_message;

/// Рендерит последовательные сообщения одного автора с закрепленной шапкой автора.
#[component]
pub(crate) fn ChatMessageGroup(
    messages: Vec<TextChatMessage>,
    appearing_message_ids: Vec<String>,
    removing_message_ids: Vec<String>,
    can_delete_messages: bool,
    on_delete: EventHandler<String>,
    server_id: String,
    room_id: String,
) -> Element {
    let Some(first_message) = messages.first().cloned() else {
        return rsx! {};
    };

    use_avatar_seed(first_message.author_user_id.clone());
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own_group = first_message.author_user_id == current_user.id;
    let avatar_column_style = if is_own_group {
        "grid-column: 3; grid-row: 1;"
    } else {
        "grid-column: 1; grid-row: 1;"
    };
    let header_class = if is_own_group {
        "mb-1.5 flex items-center justify-end gap-2 px-0.5"
    } else {
        "mb-1.5 flex items-center gap-2 px-0.5"
    };
    let sent_time = message_time(&first_message.created_at);
    let sent_datetime = full_message_datetime(&first_message.created_at);

    rsx! {
        div {
            class: "chat-message-group relative grid gap-3.5",
            style: "grid-template-columns: 2.25rem minmax(0, 1fr) 2.25rem;",
            div {
                class: "chat-message-avatar-column sticky top-3 z-20 shrink-0 self-start pt-0.5",
                style: avatar_column_style,
                UserAvatar {
                    nickname: first_message.author_nickname.clone(),
                    avatar_url: first_message.author_avatar_url.clone(),
                    class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-[12px] font-bold text-zinc-100 outline outline-1 outline-white/10".to_owned(),
                    avatar_seed: Some(first_message.author_user_id.clone()),
                }
            }
            div {
                class: "chat-message-content min-w-0",
                style: "grid-column: 2; grid-row: 1;",
                div { class: header_class,
                    span { class: "truncate text-[12px] font-semibold tracking-[-0.01em] text-zinc-100",
                        "{first_message.author_nickname}"
                    }
                    time {
                        class: "shrink-0 text-[10px] tabular-nums text-zinc-600",
                        datetime: "{first_message.created_at}",
                        title: "{sent_datetime}",
                        "{sent_time}"
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    for message in messages.iter().cloned() {
                        ChatMessageItem {
                            key: "{message.id}",
                            message: message.clone(),
                            animate: is_appearing_message(&message.id, &appearing_message_ids),
                            removing: removing_message_ids.contains(&message.id),
                            can_delete_messages,
                            on_delete: move |id| on_delete.call(id),
                            for attachment in message.attachments.iter().cloned() {
                                super::image_attachment::ChatImageAttachment {
                                    key: "{super::image_attachment::ChatImageLoadKey::render_key(&server_id, &room_id, &attachment.id)}",
                                    server_id: server_id.clone(),
                                    room_id: room_id.clone(),
                                    attachment,
                                    is_own: message.author_user_id == current_user.id,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
