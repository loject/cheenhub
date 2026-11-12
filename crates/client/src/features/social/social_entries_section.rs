//! Единый боковой список друзей и существующих личных собеседников.

use cheenhub_contracts::{
    presence::UserPresence, realtime::SocialListEntrySummary, rest::UserRelationStatus,
};
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

/// Запрос на открытие контекстного меню принятого друга.
#[derive(Clone, PartialEq)]
pub(super) struct FriendMenuRequest {
    pub(super) user_id: String,
    pub(super) nickname: String,
    pub(super) x: f64,
    pub(super) y: f64,
}

/// Показывает одну выборку друзей и собеседников с поиском и продолжением страниц.
#[component]
pub(super) fn SocialEntriesSection(
    entries: Option<Vec<SocialListEntrySummary>>,
    current_user_id: String,
    selected_conversation_id: Option<String>,
    query: String,
    loading: bool,
    loading_more: bool,
    has_more: bool,
    error: Option<String>,
    on_query_change: EventHandler<String>,
    on_find_people: EventHandler<()>,
    on_open_friend: EventHandler<String>,
    on_select_conversation: EventHandler<String>,
    on_open_menu: EventHandler<FriendMenuRequest>,
    on_load_more: EventHandler<()>,
    on_retry: EventHandler<()>,
) -> Element {
    rsx! {
        section { class: "mt-5",
            h2 { class: "px-1 text-[11px] font-semibold uppercase tracking-wide text-zinc-500", "Друзья и диалоги" }
            div { class: "relative mt-3",
                input {
                    r#type: "search",
                    class: "h-10 w-full rounded-lg border border-zinc-800 bg-zinc-900/70 px-3 text-[12px] text-zinc-100 outline-none transition placeholder:text-zinc-500 focus:border-blue-500/70",
                    "aria-label": "Поиск по друзьям и диалогам",
                    placeholder: "Поиск по друзьям и диалогам",
                    maxlength: "128",
                    value: query.clone(),
                    oninput: move |event| on_query_change.call(event.value()),
                }
            }
            if let Some(error_text) = error {
                div { class: "mt-3 rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2",
                    p { class: "text-[12px] leading-5 text-red-200", "{error_text}" }
                    button { r#type: "button", class: "mt-1 min-h-8 text-[12px] text-blue-300 hover:text-blue-200", onclick: move |_| on_retry.call(()), "Повторить" }
                }
            }
            if loading && entries.is_none() {
                div { role: "status", class: "mt-3 space-y-2", "aria-label": "Загружаем друзей и диалоги",
                    div { class: "h-14 animate-pulse rounded-lg bg-zinc-900/80" }
                    div { class: "h-14 animate-pulse rounded-lg bg-zinc-900/50" }
                }
            } else if let Some(items) = entries {
                if items.is_empty() && !loading {
                    div { class: "mt-3 rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-4 text-center",
                        if query.trim().is_empty() {
                            p { class: "text-[12px] leading-5 text-zinc-500", "Здесь появятся друзья и личные переписки." }
                            button { r#type: "button", class: "mt-2 min-h-9 text-[12px] text-blue-400 hover:text-blue-300", onclick: move |_| on_find_people.call(()), "Найти друзей" }
                        } else {
                            p { class: "text-[12px] leading-5 text-zinc-500", "Среди друзей и собеседников совпадений нет." }
                        }
                    }
                }
                div { class: "mt-2 space-y-1",
                    for (entry, preview, is_friend, online, unread_count, conversation_id, open_user_id, menu_user_id, menu_nickname) in items.into_iter().map(|entry| {
                        let preview = social_message_preview(&entry, &current_user_id);
                        let is_friend = entry.relation == Some(UserRelationStatus::Friends);
                        let online = matches!(entry.presence, Some(UserPresence::Online));
                        let unread_count = entry.conversation.as_ref().map_or(0, |conversation| conversation.unread_count);
                        let conversation_id = entry.conversation.as_ref().map(|conversation| conversation.id.clone());
                        let open_user_id = entry.user_id.clone();
                        let menu_user_id = entry.user_id.clone();
                        let menu_nickname = entry.nickname.clone();
                        (entry, preview, is_friend, online, unread_count, conversation_id, open_user_id, menu_user_id, menu_nickname)
                    }) {
                        button {
                            key: "{entry.user_id}",
                            r#type: "button",
                            "aria-label": "Открыть диалог с {entry.nickname}",
                            "aria-current": if entry.conversation.as_ref().map(|conversation| conversation.id.as_str()) == selected_conversation_id.as_deref() { "page" } else { "false" },
                            class: "group flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left transition hover:bg-zinc-900/70 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400/70 aria-[current=page]:bg-zinc-900",
                            onclick: move |_| {
                                if let Some(id) = conversation_id.as_ref() {
                                    on_select_conversation.call(id.clone());
                                } else {
                                    on_open_friend.call(open_user_id.clone());
                                }
                            },
                            oncontextmenu: move |event| {
                                if !is_friend {
                                    return;
                                }
                                event.prevent_default();
                                event.stop_propagation();
                                let point = event.client_coordinates();
                                on_open_menu.call(FriendMenuRequest {
                                    user_id: menu_user_id.clone(),
                                    nickname: menu_nickname.clone(),
                                    x: point.x,
                                    y: point.y,
                                });
                            },
                            UserAvatar {
                                nickname: entry.nickname.clone(),
                                avatar_url: entry.avatar_url,
                                avatar_seed: Some(entry.user_id.clone()),
                                class: "h-9 w-9 shrink-0 rounded-lg border border-zinc-800 bg-zinc-900 text-[12px] font-bold text-zinc-100".to_owned(),
                            }
                            span { class: "min-w-0 flex-1",
                                span { class: "flex min-w-0 items-center gap-2",
                                    span { class: "min-w-0 flex-1 truncate text-[13px] font-medium text-zinc-100", "{entry.nickname}" }
                                    if is_friend {
                                        if online {
                                            span { class: "flex shrink-0 items-center gap-1 text-[10px] text-emerald-400",
                                                span { class: "h-1.5 w-1.5 rounded-full bg-emerald-400" }
                                                "В сети"
                                            }
                                        } else {
                                            span { class: "flex shrink-0 items-center gap-1 text-[10px] text-zinc-500",
                                                span { class: "h-1.5 w-1.5 rounded-full bg-zinc-600" }
                                                "Не в сети"
                                            }
                                        }
                                    } else if entry.conversation.as_ref().is_some_and(|conversation| matches!(conversation.block_status, cheenhub_contracts::rest::DmBlockStatus::BlockedByMe | cheenhub_contracts::rest::DmBlockStatus::Mutual)) {
                                        span { class: "shrink-0 text-[10px] text-red-300", "Заблокирован" }
                                    } else if entry.conversation.as_ref().is_some_and(|conversation| conversation.block_status == cheenhub_contracts::rest::DmBlockStatus::BlockedByPeer) {
                                        span { class: "shrink-0 text-[10px] text-red-300", "Заблокировал вас" }
                                    } else {
                                        span { class: "shrink-0 text-[10px] text-amber-400/90", "Не в друзьях" }
                                    }
                                }
                                span { class: "block truncate text-[11px] text-zinc-500", "{preview}" }
                            }
                            if unread_count > 0 {
                                span {
                                    class: "dm-unread-badge shrink-0 rounded-full bg-blue-500 px-2 py-0.5 text-[10px] font-bold text-white",
                                    title: "{unread_count} непрочитанных",
                                    span { class: "dm-unread-badge-value", "{unread_badge_label(unread_count)}" }
                                }
                            }
                        }
                    }
                }
                if has_more {
                    button {
                        r#type: "button",
                        class: "mt-2 flex h-9 w-full items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900/60 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-60",
                        disabled: loading || loading_more,
                        onclick: move |_| on_load_more.call(()),
                        if loading_more { "Загрузка…" } else { "Показать ещё" }
                    }
                }
            } else if !loading {
                button { r#type: "button", class: "mt-2 min-h-10 px-1 text-[12px] text-blue-400 hover:text-blue-300", onclick: move |_| on_retry.call(()), "Загрузить список повторно" }
            }
        }
    }
}

fn social_message_preview(entry: &SocialListEntrySummary, current_user_id: &str) -> String {
    let Some(message) = &entry.last_message else {
        return "Начните общение".to_owned();
    };
    let content = if message.body.trim().is_empty() && message.has_image {
        "Изображение".to_owned()
    } else {
        message.body.trim().to_owned()
    };
    if message.sender_user_id == current_user_id {
        format!("Вы: {content}")
    } else {
        content
    }
}

fn unread_badge_label(unread_count: i64) -> String {
    if unread_count > 99 {
        "99+".to_owned()
    } else {
        unread_count.to_string()
    }
}

#[cfg(test)]
mod tests;
