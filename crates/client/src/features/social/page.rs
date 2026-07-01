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
use crate::features::text_chat::{ScrollCommand, apply_scroll_command};
use crate::features::voice_chat::VoiceConnectionHandle;

use super::api;
use super::direct_message_voice_button::DirectMessageVoiceButton;
use super::direct_message_workspace::DirectMessageWorkspace;
use super::friend_context_menu::FriendContextMenu;
use super::friend_search_modal::FriendSearchModal;
use super::friends_section::{FriendMenuRequest, FriendsSection};
use super::presentation::{
    MessageRefreshSignals, load_messages, load_social_overview, push_message_with_motion,
    refresh_conversations, refresh_friends, refresh_messages,
};
use super::realtime::{subscribe_social_events, subscribe_social_ready_events};
use super::requests_section::FriendRequestsSection;

/// Рендерит рабочую область друзей и личных сообщений.
#[component]
pub(crate) fn SocialPage() -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let realtime = use_context::<RealtimeHandle>();
    let voice = use_context::<VoiceConnectionHandle>();
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
    let list_element = use_signal(|| None::<Rc<MountedData>>);
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

    let voice_loader = voice.clone();
    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        reload.call(());
        voice_loader.load_direct_message_voice_rooms();
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
                    show_voice_controls: true,
                }
            }

            section { class: "flex min-h-0 min-w-0 flex-1 flex-col",
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
                    if let Some(conversation) = selected_conversation() {
                        DirectMessageVoiceButton { conversation }
                    }
                }
                if let Some(conversation) = selected_conversation() {
                    DirectMessageWorkspace {
                        conversation,
                        messages,
                        appearing_message_ids,
                        removing_message_ids,
                        is_loading_messages,
                        draft,
                        is_sending,
                        is_near_bottom,
                        list_element,
                        pending_scroll,
                        on_send_message: move |_| send_message.call(()),
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
