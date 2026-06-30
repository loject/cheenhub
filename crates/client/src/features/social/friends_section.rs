//! Секция друзей на social-экране.

use cheenhub_contracts::rest::{DmConversationSummary, FriendSummary};
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
    conversations: Vec<DmConversationSummary>,
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
                    for (friend, open_friend_user_id, menu_friend_user_id, menu_friend_nickname, unread_count) in friends.into_iter().map(|friend| {
                        let open_friend_user_id = friend.user_id.clone();
                        let menu_friend_user_id = friend.user_id.clone();
                        let menu_friend_nickname = friend.nickname.clone();
                        let unread_count = friend_unread_count(&friend, &conversations);
                        (friend, open_friend_user_id, menu_friend_user_id, menu_friend_nickname, unread_count)
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
                            if unread_count > 0 {
                                span {
                                    class: "shrink-0 rounded-full bg-blue-500 px-2 py-0.5 text-[10px] font-bold text-white",
                                    title: "{unread_count} непрочитанных",
                                    "{unread_badge_label(unread_count)}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn friend_unread_count(friend: &FriendSummary, conversations: &[DmConversationSummary]) -> i64 {
    conversations
        .iter()
        .find(|conversation| conversation.friend_user_id == friend.user_id)
        .map(|conversation| conversation.unread_count)
        .unwrap_or(friend.unread_count)
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
    use cheenhub_contracts::rest::{DmConversationSummary, FriendSummary};

    use super::{friend_unread_count, unread_badge_label};

    #[test]
    fn unread_badge_caps_only_display_value() {
        assert_eq!(unread_badge_label(0), "0");
        assert_eq!(unread_badge_label(99), "99");
        assert_eq!(unread_badge_label(100000), "99+");
    }

    #[test]
    fn unread_badge_prefers_conversation_counter() {
        let friend = FriendSummary {
            user_id: "friend-1".to_owned(),
            nickname: "Friend".to_owned(),
            avatar_url: None,
            unread_count: 7,
            friends_since: "2026-06-30T00:00:00Z".to_owned(),
        };
        let conversations = vec![DmConversationSummary {
            id: "conversation-1".to_owned(),
            friend_user_id: "friend-1".to_owned(),
            friend_nickname: "Friend".to_owned(),
            friend_avatar_url: None,
            unread_count: 2,
            last_read_message_id: None,
            last_read_seq: 0,
            last_read_at: None,
            updated_at: "2026-06-30T00:00:00Z".to_owned(),
        }];

        assert_eq!(friend_unread_count(&friend, &conversations), 2);
    }
}
