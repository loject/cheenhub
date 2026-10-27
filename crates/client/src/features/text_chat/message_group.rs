//! Группа последовательных сообщений одного автора в текстовом чате.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};
use crate::features::app::current_user::CurrentUserContext;
use crate::features::server_registry::ServerRegistry;

use super::message_date::full_message_datetime;
use super::message_edit_dialog::InlineMessageEditState;
use super::message_item::ChatMessageItem;
use super::message_item::message_time;
use super::messages::is_appearing_message;

/// Рендерит последовательные сообщения одного автора с закрепленной шапкой автора.
#[component]
pub(crate) fn ChatMessageGroup(
    messages: Vec<TextChatMessage>,
    #[props(default)] anchor_elements: Option<super::scroll_anchor_runtime::AnchorElements>,
    appearing_message_ids: Vec<String>,
    removing_message_ids: Vec<String>,
    can_delete_messages: bool,
    on_delete: EventHandler<String>,
    on_edit: Option<EventHandler<String>>,
    inline_edit: Option<InlineMessageEditState>,
    can_pin_messages: bool,
    pinned_message_ids: Vec<String>,
    on_pin_toggle: EventHandler<String>,
    highlighted_message_id: Option<String>,
    server_id: String,
    room_id: String,
) -> Element {
    let Some(first_message) = messages.first().cloned() else {
        return rsx! {};
    };

    use_avatar_seed(first_message.author_user_id.clone());
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own_group = first_message.author_user_id == current_user.id;
    let nickname_color = use_context::<ServerRegistry>()
        .role_color(&server_id, &first_message.author_user_id)
        .unwrap_or_else(|| if is_own_group { "#8cb8fa" } else { "#f0f2f6" }.to_owned());
    let sent_time = message_time(&first_message.created_at);
    let sent_datetime = full_message_datetime(&first_message.created_at);

    rsx! {
        div {
            class: "chat-message-group relative grid gap-3.5",
            style: "grid-template-columns: 2.5rem minmax(0, 1fr);",
            div {
                class: "chat-message-avatar-column sticky top-3 z-20 shrink-0 self-start pt-0.5",
                UserAvatar {
                    nickname: first_message.author_nickname.clone(),
                    avatar_url: first_message.author_avatar_url.clone(),
                    class: (if is_own_group { "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-[12px] font-bold text-zinc-100 ring-2 ring-blue-400/35" } else { "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-[12px] font-bold text-zinc-100 outline outline-1 outline-white/10" }).to_owned(),
                    avatar_seed: Some(first_message.author_user_id.clone()),
                }
            }
            div {
                class: "chat-message-content min-w-0",
                div { class: "mb-1 flex items-center gap-2 px-2",
                    span {
                        class: "truncate text-[13.5px] font-semibold tracking-[-0.012em]",
                        style: "color: {nickname_color};",
                        "{first_message.author_nickname}"
                    }
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
                            message: message.clone(),
                            anchor_elements,
                            animate: is_appearing_message(&message.id, &appearing_message_ids),
                            removing: removing_message_ids.contains(&message.id),
                            show_hover_time: index > 0,
                            allow_own_delete: true,
                            can_delete_messages,
                            on_delete: move |id| on_delete.call(id),
                            on_edit,
                            inline_edit: inline_edit.clone(),
                            can_pin_messages,
                            is_pinned: pinned_message_ids.contains(&message.id),
                            on_pin_toggle: Some(on_pin_toggle),
                            highlighted: highlighted_message_id.as_deref() == Some(message.id.as_str()),
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
