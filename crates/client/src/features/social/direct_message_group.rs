//! Группа последовательных личных сообщений одного автора.

use cheenhub_contracts::rest::DmMessageSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};
use crate::features::app::current_user::CurrentUserContext;
use crate::features::text_chat::{ChatMessageItem, is_appearing_message};

use super::direct_message_image::DirectMessageImage;
use super::presentation::dm_as_text_message;

/// Рендерит сообщения и изображения в исходном порядке внутри авторской группы.
#[component]
pub(super) fn DirectMessageGroup(
    messages: Vec<DmMessageSummary>,
    appearing_message_ids: Vec<String>,
    removing_message_ids: Vec<String>,
) -> Element {
    let Some(first_message) = messages.first().cloned() else {
        return rsx! {};
    };

    use_avatar_seed(first_message.sender_user_id.clone());
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own_group = first_message.sender_user_id == current_user.id;
    let avatar_column_style = if is_own_group {
        "grid-column: 3; grid-row: 1;"
    } else {
        "grid-column: 1; grid-row: 1;"
    };
    let header_class = if is_own_group {
        "mb-1 flex items-center justify-end gap-2"
    } else {
        "mb-1 flex items-center gap-2"
    };

    rsx! {
        div {
            class: "chat-message-group relative grid gap-3",
            style: "grid-template-columns: 2.25rem minmax(0, 1fr) 2.25rem;",
            div {
                class: "chat-message-avatar-column sticky top-0 z-20 shrink-0 self-start pt-1",
                style: avatar_column_style,
                UserAvatar {
                    nickname: first_message.sender_nickname.clone(),
                    avatar_url: first_message.sender_avatar_url.clone(),
                    class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100".to_owned(),
                    avatar_seed: Some(first_message.sender_user_id.clone()),
                }
            }
            div {
                class: "chat-message-content min-w-0",
                style: "grid-column: 2; grid-row: 1;",
                div { class: header_class,
                    span { class: "truncate text-[12px] font-semibold text-zinc-100", "{first_message.sender_nickname}" }
                }
                div { class: "flex flex-col gap-2",
                    for message in messages.iter().cloned() {
                        div { key: "{message.id}", class: "contents",
                            ChatMessageItem {
                                message: dm_as_text_message(message.clone()),
                                animate: is_appearing_message(&message.id, &appearing_message_ids),
                                removing: removing_message_ids.contains(&message.id),
                                can_delete_messages: false,
                                on_delete: move |_| {},
                            }
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
