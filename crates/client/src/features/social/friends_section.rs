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
    is_loading: bool,
    on_search: EventHandler<()>,
    on_open_friend: EventHandler<String>,
    on_open_menu: EventHandler<FriendMenuRequest>,
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
                    for (friend, open_friend_user_id, menu_friend_user_id, menu_friend_nickname) in friends.into_iter().map(|friend| {
                        let open_friend_user_id = friend.user_id.clone();
                        let menu_friend_user_id = friend.user_id.clone();
                        let menu_friend_nickname = friend.nickname.clone();
                        (friend, open_friend_user_id, menu_friend_user_id, menu_friend_nickname)
                    }) {
                        div {
                            key: "{friend.user_id}",
                            class: "group flex items-center gap-2 rounded-lg px-2 py-2 hover:bg-zinc-900/70",
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
                            button {
                                r#type: "button",
                                class: "min-w-0 flex-1 text-left",
                                onclick: move |_| on_open_friend.call(open_friend_user_id.clone()),
                                p { class: "truncate text-[13px] font-medium text-zinc-100", "{friend.nickname}" }
                                p { class: "text-[11px] text-zinc-500", "Открыть диалог" }
                            }
                        }
                    }
                }
            }
        }
    }
}
