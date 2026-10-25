//! Список личных диалогов, включая переписку с пользователями вне списка друзей.

use cheenhub_contracts::rest::DmConversationSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

/// Показывает диалоги и непрочитанные сообщения независимо от дружбы.
#[component]
pub(super) fn ConversationsSection(
    conversations: Option<Vec<DmConversationSummary>>,
    selected_id: Option<String>,
    loading: bool,
    on_select: EventHandler<String>,
    on_find_people: EventHandler<()>,
    on_retry: EventHandler<()>,
) -> Element {
    rsx! {
        section { class: "mt-5",
            h2 { class: "px-1 text-[11px] font-semibold uppercase tracking-wide text-zinc-500", "Личные диалоги" }
            if loading && conversations.is_none() {
                div { role: "status", class: "mt-2 h-12 animate-pulse rounded-lg bg-zinc-900/80", "aria-label": "Загружаем диалоги" }
            } else if let Some(items) = conversations {
                if items.is_empty() {
                    p { class: "px-1 pt-2 text-[12px] leading-5 text-zinc-500", "Здесь появится ваша личная переписка." }
                    button { r#type: "button", class: "min-h-10 px-1 text-[12px] text-blue-400 hover:text-blue-300", onclick: move |_| on_find_people.call(()), "Найти собеседника" }
                }
                div { class: "mt-2 space-y-1",
                    for conversation in items {
                        button {
                            key: "{conversation.id}",
                            r#type: "button",
                            "aria-current": if selected_id.as_deref() == Some(conversation.id.as_str()) { "page" } else { "false" },
                            class: "flex min-h-12 w-full items-center gap-2.5 rounded-xl px-2 py-2 text-left transition-[background-color] duration-150 hover:bg-zinc-900 aria-[current=page]:bg-zinc-900",
                            onclick: {
                                let id = conversation.id.clone();
                                move |_| on_select.call(id.clone())
                            },
                            UserAvatar {
                                nickname: conversation.friend_nickname.clone(),
                                avatar_url: conversation.friend_avatar_url,
                                avatar_seed: Some(conversation.friend_user_id),
                                class: "h-8 w-8 shrink-0 rounded-full bg-zinc-800 text-[11px] font-semibold text-zinc-200".to_owned(),
                            }
                            span { class: "min-w-0 flex-1 truncate text-[13px] font-medium text-zinc-200", "{conversation.friend_nickname}" }
                            if conversation.unread_count > 0 {
                                span { class: "dm-unread-badge", title: "{conversation.unread_count} непрочитанных",
                                    span { class: "dm-unread-badge-value",
                                        if conversation.unread_count > 99 { "99+" } else { "{conversation.unread_count}" }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                button { r#type: "button", class: "min-h-10 px-1 text-[12px] text-blue-400 hover:text-blue-300", onclick: move |_| on_retry.call(()), "Загрузить диалоги повторно" }
            }
        }
    }
}
