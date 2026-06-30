//! Экран друзей и личных сообщений.

use std::rc::Rc;

use cheenhub_contracts::{
    realtime::SocialChangeReason,
    rest::{DmConversationSummary, DmMessageSummary, FriendRequestSummary, FriendSummary},
};
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::components::app_sidebar_footer::AppSidebarFooter;
use crate::features::app::components::avatar::UserAvatar;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::realtime::RealtimeHandle;
use crate::features::text_chat::{
    CHAT_COMPOSER_CLASS, CHAT_CONTENT_CLASS, ChatMessageItem, ScrollCommand, apply_scroll_command,
    update_near_bottom_state,
};

use super::api;
use super::friend_context_menu::FriendContextMenu;
use super::friend_search_modal::FriendSearchModal;
use super::friends_section::{FriendMenuRequest, FriendsSection};
use super::presentation::{
    MessageRefreshSignals, dm_as_text_message, is_appearing_message, load_messages,
    load_social_overview, push_message_with_motion, refresh_conversations, refresh_friends,
    refresh_messages,
};
use super::realtime::{subscribe_social_events, subscribe_social_ready_events};
use super::requests_section::FriendRequestsSection;

/// Рендерит рабочую область друзей и личных сообщений.
#[component]
pub(crate) fn SocialPage() -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let realtime = use_context::<RealtimeHandle>();
    let friends = use_signal(Vec::<FriendSummary>::new);
    let incoming = use_signal(Vec::<FriendRequestSummary>::new);
    let outgoing = use_signal(Vec::<FriendRequestSummary>::new);
    let mut conversations = use_signal(Vec::<DmConversationSummary>::new);
    let mut selected_conversation = use_signal(|| None::<DmConversationSummary>);
    let messages = use_signal(Vec::<DmMessageSummary>::new);
    let appearing_message_ids = use_signal(Vec::<String>::new);
    let removing_message_ids = use_signal(Vec::<String>::new);
    let mut draft = use_signal(String::new);
    let mut status = use_signal(String::new);
    let is_loading = use_signal(|| false);
    let is_loading_messages = use_signal(|| false);
    let mut is_sending = use_signal(|| false);
    let mut is_search_open = use_signal(|| false);
    let mut loaded = use_signal(|| false);
    let mut requests_collapsed = use_signal(|| true);
    let mut friend_menu = use_signal(|| None::<FriendMenuRequest>);
    let is_near_bottom = use_signal(|| true);
    let mut list_element = use_signal(|| None::<Rc<MountedData>>);
    let mut pending_scroll = use_signal(|| None::<ScrollCommand>);

    let reload = use_callback(move |_| {
        load_social_overview(
            friends,
            incoming,
            outgoing,
            conversations,
            status,
            is_loading,
        );
    });

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        reload.call(());
    });

    let open_friend = use_callback(move |friend_user_id: String| {
        status.set(String::new());
        spawn(async move {
            match api::open_dm_conversation(friend_user_id).await {
                Ok(conversation) => {
                    let conversation_id = conversation.id.clone();
                    selected_conversation.set(Some(conversation));
                    load_messages(
                        conversation_id,
                        messages,
                        conversations,
                        friends,
                        status,
                        is_loading_messages,
                        pending_scroll,
                    );
                    if let Ok(next_conversations) = api::list_dm_conversations().await {
                        conversations.set(next_conversations);
                    }
                }
                Err(error) => status.set(error),
            }
        });
    });

    let delete_friend = use_callback(move |friend_user_id: String| {
        spawn(async move {
            info!(%friend_user_id, "removing friend from social context menu");
            match api::delete_friend(&friend_user_id).await {
                Ok(()) => reload.call(()),
                Err(error) => {
                    warn!(%friend_user_id, %error, "failed to remove friend");
                    status.set(error);
                }
            }
        });
    });

    let accept_request = use_callback(move |request_id: String| {
        spawn(async move {
            match api::accept_friend_request(&request_id).await {
                Ok(_) => reload.call(()),
                Err(error) => status.set(error),
            }
        });
    });

    let decline_request = use_callback(move |request_id: String| {
        spawn(async move {
            match api::decline_friend_request(&request_id).await {
                Ok(_) => reload.call(()),
                Err(error) => status.set(error),
            }
        });
    });

    let cancel_request = use_callback(move |request_id: String| {
        spawn(async move {
            match api::cancel_friend_request(&request_id).await {
                Ok(_) => reload.call(()),
                Err(error) => status.set(error),
            }
        });
    });

    let send_message = use_callback(move |_| {
        let Some(conversation) = selected_conversation() else {
            return;
        };
        let body = draft().trim().to_owned();
        if body.is_empty() || is_sending() {
            return;
        }

        is_sending.set(true);
        status.set(String::new());
        spawn(async move {
            match api::send_dm_message(&conversation.id, body).await {
                Ok(message) => {
                    push_message_with_motion(messages, appearing_message_ids, message);
                    if is_near_bottom() {
                        pending_scroll.set(Some(ScrollCommand::Bottom));
                    }
                    draft.set(String::new());
                    if let Ok(next_conversations) = api::list_dm_conversations().await {
                        conversations.set(next_conversations);
                    }
                }
                Err(error) => status.set(error),
            }
            is_sending.set(false);
        });
    });

    let refresh_direct_state = use_callback(move |changed_conversation_id: Option<String>| {
        let selected = selected_conversation();
        let selected_conversation_id = selected
            .as_ref()
            .map(|conversation| conversation.id.as_str());
        debug!(
            selected_conversation_id = ?selected_conversation_id,
            changed_conversation_id = ?changed_conversation_id,
            "refreshing social state after direct message change"
        );
        refresh_conversations(conversations, status);
        refresh_friends(friends, status);
        if let Some(conversation) = selected {
            let should_refresh_messages = changed_conversation_id
                .as_deref()
                .is_none_or(|changed_id| changed_id == conversation.id.as_str());
            if should_refresh_messages {
                refresh_messages(
                    conversation.id,
                    MessageRefreshSignals {
                        messages,
                        conversations,
                        friends,
                        appearing_message_ids,
                        removing_message_ids,
                        status,
                        is_near_bottom,
                        pending_scroll,
                    },
                );
            }
        }
    });

    use_hook(move || {
        let realtime = realtime.clone();
        let ready_realtime = realtime.clone();
        let ready_refresh = refresh_direct_state;
        spawn(async move {
            let mut ready = subscribe_social_ready_events(ready_realtime);
            while ready.next().await.is_some() {
                debug!("refreshing social state after realtime subscription became active");
                ready_refresh.call(None);
            }
        });

        spawn(async move {
            let mut receiver = subscribe_social_events(&realtime);
            while let Some(event) = receiver.next().await {
                debug!(
                    reason = ?event.reason,
                    conversation_id = ?event.conversation_id,
                    "received social realtime change"
                );
                match event.reason {
                    SocialChangeReason::Friends => reload.call(()),
                    SocialChangeReason::DirectMessages => {
                        refresh_direct_state.call(event.conversation_id);
                    }
                }
            }
        });
    });

    use_effect(move || {
        let _message_count = messages.len();
        let Some(command) = pending_scroll() else {
            return;
        };
        pending_scroll.set(None);
        let Some(element) = list_element.cloned() else {
            return;
        };

        spawn(async move {
            apply_scroll_command(element, command).await;
        });
    });

    let requests_count = incoming().len() + outgoing().len();
    let appearing_message_ids_list = appearing_message_ids();
    let removing_message_ids_list = removing_message_ids();

    rsx! {
        section { class: "flex h-full min-h-0 flex-1 bg-zinc-950 text-zinc-100",
            aside { class: "flex w-[284px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/90",
                div { class: "flex h-16 shrink-0 items-center justify-between gap-3 border-b border-zinc-800/80 px-4",
                    div { class: "min-w-0",
                        h1 { class: "text-[15px] font-semibold text-zinc-50", "Друзья" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                            "Список друзей и заявки"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-blue-500 text-white transition hover:bg-blue-400",
                        "aria-label": "Найти друзей",
                        onclick: move |_| is_search_open.set(true),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m21 21-4.3-4.3M10.5 18a7.5 7.5 0 1 1 0-15 7.5 7.5 0 0 1 0 15Z" }
                        }
                    }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto p-3",
                    if !status().is_empty() {
                        p { class: "mb-3 rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                            "{status()}"
                        }
                    }

                    FriendRequestsSection {
                        incoming: incoming(),
                        outgoing: outgoing(),
                        collapsed: requests_collapsed(),
                        requests_count,
                        on_toggle: move |_| requests_collapsed.set(!requests_collapsed()),
                        on_accept: move |request_id| accept_request.call(request_id),
                        on_decline: move |request_id| decline_request.call(request_id),
                        on_cancel: move |request_id| cancel_request.call(request_id),
                    }

                    FriendsSection {
                        friends: friends(),
                        conversations: conversations(),
                        is_loading: is_loading(),
                        on_search: move |_| is_search_open.set(true),
                        on_open_friend: move |friend_user_id| {
                            friend_menu.set(None);
                            open_friend.call(friend_user_id);
                        },
                        on_open_menu: move |menu| friend_menu.set(Some(menu)),
                    }
                }
                AppSidebarFooter {
                    realtime_label: "Друзья".to_owned(),
                    settings_workspace_active: false,
                    show_voice_controls: false,
                }
            }

            section { class: "flex min-w-0 flex-1 flex-col",
                div { class: "flex min-h-16 shrink-0 items-center gap-3 border-b border-zinc-800/80 px-5",
                    if let Some(conversation) = selected_conversation() {
                        UserAvatar {
                            nickname: conversation.friend_nickname.clone(),
                            avatar_url: conversation.friend_avatar_url.clone(),
                            class: "h-9 w-9 shrink-0 rounded-xl border border-zinc-800 bg-zinc-900 text-[12px] font-bold text-zinc-100".to_owned(),
                            avatar_seed: Some(conversation.friend_user_id.clone()),
                        }
                        div { class: "min-w-0 flex-1",
                            h2 { class: "truncate text-[15px] font-semibold text-zinc-50", "{conversation.friend_nickname}" }
                            p { class: "text-[12px] text-zinc-500", "Личные сообщения" }
                        }
                    } else {
                        div { class: "min-w-0 flex-1",
                            h2 { class: "text-[15px] font-semibold text-zinc-50", "Сообщения" }
                            p { class: "text-[12px] text-zinc-500", "Личные диалоги" }
                        }
                    }
                    button {
                        r#type: "button",
                        disabled: true,
                        class: "flex h-9 shrink-0 cursor-not-allowed items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900/60 px-3 text-[12px] font-medium text-zinc-500 opacity-75",
                        "aria-label": "Звонок в разработке",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.25 6.75c0 8.284 6.716 15 15 15h2.25a2.25 2.25 0 0 0 2.25-2.25v-1.372c0-.516-.351-.966-.852-1.091l-4.423-1.106c-.44-.11-.902.055-1.173.417l-.97 1.293c-.282.376-.769.542-1.21.38a12.035 12.035 0 0 1-7.143-7.143c-.162-.441.004-.928.38-1.21l1.293-.97c.362-.271.527-.734.417-1.173L6.963 3.102A1.125 1.125 0 0 0 5.872 2.25H4.5A2.25 2.25 0 0 0 2.25 4.5v2.25Z" }
                        }
                        "Звонок в разработке"
                    }
                }

                if let Some(conversation) = selected_conversation() {
                    div {
                        class: "min-h-0 flex-1 overflow-y-auto p-5 lg:p-6",
                        onmounted: move |event| list_element.set(Some(event.data.clone())),
                        onscroll: move |_| {
                            if let Some(element) = list_element.cloned() {
                                spawn(async move {
                                    update_near_bottom_state(element, is_near_bottom).await;
                                });
                            }
                        },
                        div { class: CHAT_CONTENT_CLASS,
                            if is_loading_messages() {
                                div { class: "space-y-3",
                                    div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/80" }
                                    div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/50" }
                                }
                            } else if messages().is_empty() {
                                div { class: "rounded-[20px] border border-zinc-800 bg-zinc-900/60 p-6 text-center",
                                    p { class: "text-[13px] font-medium text-zinc-100", "Сообщений пока нет" }
                                    p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Напишите первое личное сообщение." }
                                }
                            } else {
                                for message in messages() {
                                    ChatMessageItem {
                                        key: "{message.id}",
                                        message: dm_as_text_message(message.clone()),
                                        animate: is_appearing_message(
                                            &message.id,
                                            &appearing_message_ids_list,
                                        ),
                                        removing: removing_message_ids_list.contains(&message.id),
                                        can_delete_messages: false,
                                        on_delete: move |_| {},
                                    }
                                }
                            }
                        }
                    }
                    div { class: "relative",
                        if !is_near_bottom() && !messages().is_empty() {
                            div { class: "pointer-events-none absolute bottom-3 right-4 z-20",
                                button {
                                    r#type: "button",
                                    class: "group pointer-events-auto relative flex h-10 w-10 items-center justify-center rounded-full border border-zinc-800 bg-zinc-950/85 text-blue-200 shadow-[0_8px_22px_rgba(0,0,0,0.35)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-white/15 hover:bg-zinc-900/90 hover:text-blue-100",
                                    "aria-label": "Перейти к последнему сообщению",
                                    onclick: move |_| pending_scroll.set(Some(ScrollCommand::SmoothBottom)),
                                    span { class: "pointer-events-none absolute bottom-[calc(100%+8px)] right-0 whitespace-nowrap rounded-lg border border-zinc-800 bg-zinc-950/95 px-2 py-1 text-[11px] font-medium text-zinc-300 opacity-0 shadow-[0_8px_22px_rgba(0,0,0,0.35)] transition-[opacity,transform] duration-150 group-hover:opacity-100",
                                        "К последнему сообщению"
                                    }
                                    svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m0 0 6-6m-6 6-6-6" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "shrink-0 border-t border-zinc-800/80 bg-zinc-950/55 p-4 backdrop-blur-xl",
                        div { class: CHAT_COMPOSER_CLASS,
                            textarea {
                                rows: "1",
                                value: "{draft()}",
                                placeholder: "Сообщение для {conversation.friend_nickname}",
                                class: "max-h-28 min-h-10 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600",
                                oninput: move |event| draft.set(event.value()),
                                onkeydown: move |event| {
                                    if event.key() == Key::Enter && !event.modifiers().shift() {
                                        event.prevent_default();
                                        send_message.call(());
                                    }
                                },
                            }
                            button {
                                r#type: "button",
                                disabled: draft().trim().is_empty() || is_sending(),
                                class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-blue-500 text-white transition hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-45",
                                "aria-label": "Отправить сообщение",
                                onclick: move |_| send_message.call(()),
                                if is_sending() {
                                    span { class: "h-4 w-4 animate-spin rounded-full border-2 border-blue-200/40 border-t-white" }
                                } else {
                                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "flex h-full items-center justify-center px-6 text-center",
                        div { class: "max-w-sm",
                            h2 { class: "text-[16px] font-semibold text-zinc-100", "Выберите диалог" }
                            p { class: "mt-2 text-[13px] leading-6 text-zinc-500",
                                "Откройте личный диалог из списка друзей или продолжите существующую переписку."
                            }
                            p { class: "mt-5 text-[12px] text-zinc-600", "Вы вошли как {current_user.nickname}" }
                        }
                    }
                }
            }

            if is_search_open() {
                FriendSearchModal {
                    on_close: move |_| is_search_open.set(false),
                    on_changed: move |_| reload.call(()),
                }
            }
            if let Some(menu) = friend_menu() {
                FriendContextMenu {
                    friend_user_id: menu.user_id,
                    friend_nickname: menu.nickname,
                    x: menu.x,
                    y: menu.y,
                    on_close: move |_| friend_menu.set(None),
                    on_delete: move |friend_user_id| delete_friend.call(friend_user_id),
                }
            }
        }
    }
}
