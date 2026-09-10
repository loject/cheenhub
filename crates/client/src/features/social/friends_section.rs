//! Секция друзей на social-экране.

use cheenhub_contracts::rest::FriendSummary;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

/// Запрос на открытие контекстного меню друга.
#[derive(Clone, PartialEq)]
pub(super) struct FriendMenuRequest {
    /// Идентификатор друга.
    pub(super) user_id: String,
    /// Никнейм друга.
    pub(super) nickname: String,
    /// Горизонтальная координата меню.
    pub(super) x: f64,
    /// Вертикальная координата меню.
    pub(super) y: f64,
}

/// Рендерит список друзей с открытием ЛС и контекстным меню по ПКМ.
#[component]
pub(super) fn FriendsSection(
    friends: Vec<FriendSummary>,
    current_user_id: String,
    is_loading: bool,
    has_more: bool,
    is_loading_more: bool,
    on_search: EventHandler<()>,
    on_open_friend: EventHandler<String>,
    on_open_menu: EventHandler<FriendMenuRequest>,
    on_load_more: EventHandler<()>,
) -> Element {
    rsx! {
        section { class: "mt-5",
            h2 { class: "px-1 text-[11px] font-semibold uppercase tracking-wide text-zinc-500", "Друзья" }
            if is_loading && friends.is_empty() {
                div { class: "mt-2 space-y-2",
                    div { class: "h-12 animate-pulse rounded-lg bg-zinc-900/80" }
                    div { class: "h-12 animate-pulse rounded-lg bg-zinc-900/50" }
                }
            } else if friends.is_empty() {
                div { class: "mt-2 rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-4 text-center",
                    p { class: "text-[12px] leading-5 text-zinc-500",
                        "Добавьте друга через поиск, чтобы открыть личный диалог."
                    }
                    button {
                        r#type: "button",
                        class: "mt-3 h-8 rounded-md bg-blue-500 px-3 text-[12px] font-medium text-white transition hover:bg-blue-400",
                        onclick: move |_| on_search.call(()),
                        "Найти друзей"
                    }
                }
            } else {
                div { class: "mt-2 space-y-1",
                    for (friend, open_friend_user_id, menu_friend_user_id, menu_friend_nickname, preview) in friends.into_iter().map(|friend| {
                        let open_friend_user_id = friend.user_id.clone();
                        let menu_friend_user_id = friend.user_id.clone();
                        let menu_friend_nickname = friend.nickname.clone();
                        let preview = friend_message_preview(&friend, &current_user_id);
                        (friend, open_friend_user_id, menu_friend_user_id, menu_friend_nickname, preview)
                    }) {
                        button {
                            key: "{friend.user_id}",
                            r#type: "button",
                            class: "group flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left transition hover:bg-zinc-900/70 focus-visible:outline focus-visible:outline-2 focus-visible:outline-blue-400/70",
                            "aria-label": "Открыть диалог с {friend.nickname}",
                            onclick: move |_| {
                                debug!(
                                    friend_user_id = %open_friend_user_id,
                                    "opening direct message from friend list"
                                );
                                on_open_friend.call(open_friend_user_id.clone());
                            },
                            oncontextmenu: move |event| {
                                event.prevent_default();
                                event.stop_propagation();
                                let point = event.client_coordinates();
                                debug!(
                                    friend_user_id = %menu_friend_user_id,
                                    "opened friend context menu"
                                );
                                on_open_menu.call(FriendMenuRequest {
                                    user_id: menu_friend_user_id.clone(),
                                    nickname: menu_friend_nickname.clone(),
                                    x: point.x,
                                    y: point.y,
                                });
                            },
                            UserAvatar {
                                nickname: friend.nickname.clone(),
                                avatar_url: friend.avatar_url.clone(),
                                class: "h-9 w-9 shrink-0 rounded-lg border border-zinc-800 bg-zinc-900 text-[12px] font-bold text-zinc-100".to_owned(),
                                avatar_seed: Some(friend.user_id.clone()),
                            }
                            span { class: "min-w-0 flex-1",
                                span { class: "block truncate text-[13px] font-medium text-zinc-100", "{friend.nickname}" }
                                span { class: "block truncate text-[11px] text-zinc-500", "{preview}" }
                            }
                            if friend.unread_count > 0 {
                                span {
                                    key: "unread-badge-{friend.user_id}-{friend.unread_count}",
                                    class: "dm-unread-badge shrink-0 rounded-full bg-blue-500 px-2 py-0.5 text-[10px] font-bold text-white",
                                    title: "{friend.unread_count} непрочитанных",
                                    span { class: "dm-unread-badge-value", "{unread_badge_label(friend.unread_count)}" }
                                }
                            }
                        }
                    }
                    if has_more {
                        button {
                            r#type: "button",
                            class: "mt-2 flex h-9 w-full items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900/60 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 disabled:cursor-wait disabled:opacity-60",
                            disabled: is_loading_more,
                            onclick: move |_| on_load_more.call(()),
                            if is_loading_more { "Загрузка…" } else { "Показать ещё" }
                        }
                    }
                }
            }
        }
    }
}

fn friend_message_preview(friend: &FriendSummary, current_user_id: &str) -> String {
    let Some(message) = &friend.last_message else {
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
mod tests {
    use cheenhub_contracts::rest::{DmLastMessageSummary, FriendSummary};

    use super::{friend_message_preview, unread_badge_label};

    #[test]
    fn unread_badge_caps_only_display_value() {
        assert_eq!(unread_badge_label(0), "0");
        assert_eq!(unread_badge_label(99), "99");
        assert_eq!(unread_badge_label(100000), "99+");
    }

    #[test]
    fn message_preview_marks_own_message_and_image_only_message() {
        let mut friend = FriendSummary {
            user_id: "friend-1".to_owned(),
            nickname: "Friend".to_owned(),
            avatar_url: None,
            unread_count: 0,
            last_message: Some(DmLastMessageSummary {
                id: "message-1".to_owned(),
                sender_user_id: "current-user".to_owned(),
                body: " Ответ ".to_owned(),
                has_image: false,
                created_at: "2026-09-10T00:00:00Z".to_owned(),
            }),
            friends_since: "2026-06-30T00:00:00Z".to_owned(),
        };
        assert_eq!(friend_message_preview(&friend, "current-user"), "Вы: Ответ");

        let message = friend
            .last_message
            .as_mut()
            .expect("last message should exist");
        message.sender_user_id = "friend-1".to_owned();
        message.body.clear();
        message.has_image = true;
        assert_eq!(
            friend_message_preview(&friend, "current-user"),
            "Изображение"
        );
    }
}
