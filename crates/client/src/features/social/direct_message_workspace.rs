//! Рабочая область выбранного личного диалога.

use std::rc::Rc;

use cheenhub_contracts::rest::{DmConversationSummary, DmMessageSummary};
use dioxus::prelude::*;

use crate::features::app::components::workspace_split::{
    EMBEDDED_CHAT_DEFAULT_WORKSPACE_RATIO, clamp_embedded_chat_height, finish_embedded_chat_resize,
};
use crate::features::app::current_user::CurrentUserContext;
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};
use crate::features::text_chat::{
    CHAT_COMPOSER_CLASS, CHAT_CONTENT_CLASS, ChatMessageItem, ScrollCommand,
    update_near_bottom_state,
};
use crate::features::voice_chat::{
    VoiceConnectionHandle, VoiceConnectionState, VoiceControls, VoiceParticipantGrid,
    VoiceParticipantGridStatus,
};

use super::presentation::{dm_as_text_message, is_appearing_message};
use super::voice_target::direct_message_voice_target;

/// Рендерит сообщения и голосовую область выбранного личного диалога.
#[component]
pub(crate) fn DirectMessageWorkspace(
    conversation: DmConversationSummary,
    messages: Signal<Vec<DmMessageSummary>>,
    appearing_message_ids: Signal<Vec<String>>,
    removing_message_ids: Signal<Vec<String>>,
    is_loading_messages: Signal<bool>,
    mut draft: Signal<String>,
    is_sending: Signal<bool>,
    is_near_bottom: Signal<bool>,
    mut list_element: Signal<Option<Rc<MountedData>>>,
    mut pending_scroll: Signal<Option<ScrollCommand>>,
    on_send_message: EventHandler<()>,
) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let voice = use_context::<VoiceConnectionHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let mut embedded_chat_height_px = use_signal(|| None::<f64>);
    let mut embedded_chat_resize_origin = use_signal(|| None::<(f64, f64, f64)>);
    let mut content_split_element = use_signal(|| None::<Rc<MountedData>>);
    let mut direct_workspace_conversation_id = use_signal(|| None::<String>);
    let target = direct_message_voice_target(&conversation);
    let voice_state = voice.state();
    let selected_voice_active = voice_state
        .active_target()
        .is_some_and(|active| active.matches(&target));
    let selected_voice_connected = matches!(
        &voice_state,
        VoiceConnectionState::Connected {
            target: connected_target,
            ..
        } if connected_target.matches(&target)
    );
    let selected_voice_participants = if selected_voice_active {
        voice_state.participants().to_vec()
    } else {
        Vec::new()
    };
    let selected_voice_status = match &voice_state {
        VoiceConnectionState::Connecting {
            target: connecting_target,
        } if connecting_target.matches(&target) => VoiceParticipantGridStatus::Connecting,
        VoiceConnectionState::Error {
            target: error_target,
            message,
        } if error_target
            .as_ref()
            .is_some_and(|error_target| error_target.matches(&target)) =>
        {
            VoiceParticipantGridStatus::Error {
                message: message.clone(),
            }
        }
        _ => VoiceParticipantGridStatus::Empty,
    };
    let mut selected_voice_speaking_user_ids = if selected_voice_active {
        voice.speaking_user_ids()
    } else {
        Vec::new()
    };
    if selected_voice_active
        && matches!(microphone.status(), MicrophoneStatus::Live)
        && microphone.level().active
        && !selected_voice_speaking_user_ids
            .iter()
            .any(|user_id| user_id == &current_user.id)
    {
        selected_voice_speaking_user_ids.push(current_user.id.clone());
    }
    let chat_resizing = embedded_chat_resize_origin().is_some();
    let chat_resizing_attr = if chat_resizing { "true" } else { "false" };
    let workspace_style = embedded_chat_height_px()
        .map(|height_px| format!("--embedded-chat-height: {}px;", height_px.round()))
        .unwrap_or_default();
    let direct_chat_surface_class = if selected_voice_active {
        "embedded-chat h-0 shrink-0 translate-y-6 overflow-hidden border-t border-transparent bg-[rgba(9,9,11,.86)] opacity-0 shadow-[0_-1px_0_rgba(255,255,255,0.025),0_-24px_70px_rgba(0,0,0,0.22)] backdrop-blur-[18px] transition-[height,opacity,transform,border-color] duration-[340ms] ease-[cubic-bezier(0.22,1,0.36,1)]"
    } else {
        "flex min-h-0 flex-1 flex-col"
    };
    let direct_chat_inner_class = if selected_voice_active {
        "flex h-full min-h-0 flex-col"
    } else {
        "flex min-h-0 flex-1 flex-col"
    };
    let appearing_message_ids_list = appearing_message_ids();
    let removing_message_ids_list = removing_message_ids();
    let retry_voice = voice.clone();
    let conversation_id = conversation.id.clone();

    use_effect(move || {
        let next_conversation_id = Some(conversation_id.clone());
        if direct_workspace_conversation_id() == next_conversation_id {
            return;
        }

        direct_workspace_conversation_id.set(next_conversation_id);
        embedded_chat_height_px.set(None);
        embedded_chat_resize_origin.set(None);
    });

    rsx! {
        section {
            class: "room-workspace voice-shell relative flex min-h-0 flex-1 flex-col bg-zinc-950/35",
            style: "{workspace_style}",
            "data-room-kind": "direct",
            "data-voice-room-active": if selected_voice_active { "true" } else { "false" },
            "data-voice-connected": if selected_voice_connected { "true" } else { "false" },
            div {
                class: "content-split flex min-h-0 flex-1 flex-col",
                "data-chat-resizing": chat_resizing_attr,
                onmounted: move |event| content_split_element.set(Some(event.data.clone())),
                onpointermove: move |event| {
                    let Some((start_y, start_height, workspace_height)) = embedded_chat_resize_origin() else {
                        return;
                    };

                    event.prevent_default();
                    let point = event.client_coordinates();
                    let next_height = clamp_embedded_chat_height(
                        start_height + start_y - point.y,
                        workspace_height,
                    );
                    embedded_chat_height_px.set(Some(next_height));
                },
                onpointerup: {
                    let conversation_id = conversation.id.clone();
                    move |_| {
                        finish_embedded_chat_resize(
                            embedded_chat_resize_origin,
                            embedded_chat_height_px,
                            &conversation_id,
                        );
                    }
                },
                onpointerleave: {
                    let conversation_id = conversation.id.clone();
                    move |_| {
                        finish_embedded_chat_resize(
                            embedded_chat_resize_origin,
                            embedded_chat_height_px,
                            &conversation_id,
                        );
                    }
                },
                if selected_voice_active {
                    div { class: "voice-room-surface relative flex min-h-0 flex-1 flex-col",
                        VoiceParticipantGrid {
                            server_id: target.server_id.clone(),
                            room_id: target.room_id.clone(),
                            participants: selected_voice_participants.clone(),
                            speaking_user_ids: selected_voice_speaking_user_ids.clone(),
                            status: selected_voice_status.clone(),
                            can_kick_voice: false,
                            on_retry: {
                                let retry_voice = retry_voice.clone();
                                let retry_target = target.clone();
                                move |_| retry_voice.join(retry_target.clone())
                            },
                        }
                        VoiceControls { target: target.clone() }
                    }
                }
                div {
                    class: direct_chat_surface_class,
                    "data-resizing": chat_resizing_attr,
                    div { class: direct_chat_inner_class,
                        if selected_voice_active {
                            div {
                                class: "chat-resize-handle flex h-3.5 shrink-0 cursor-ns-resize touch-none items-center justify-center",
                                role: "separator",
                                "aria-orientation": "horizontal",
                                "aria-label": "Потяните, чтобы изменить высоту чата",
                                onpointerdown: move |event| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    let point = event.client_coordinates();
                                    let split_element = content_split_element.cloned();
                                    let measured_height_px = embedded_chat_height_px();

                                    spawn(async move {
                                        let workspace_height = match split_element {
                                            Some(element) => element
                                                .get_client_rect()
                                                .await
                                                .ok()
                                                .map(|rect| rect.size.height),
                                            None => None,
                                        }
                                        .filter(|height| *height > 0.0)
                                        .unwrap_or_else(|| {
                                            measured_height_px
                                                .filter(|height| *height > 0.0)
                                                .unwrap_or(1.0)
                                                / EMBEDDED_CHAT_DEFAULT_WORKSPACE_RATIO
                                        });
                                        let start_height = measured_height_px
                                            .filter(|height_px| *height_px > 0.0)
                                            .unwrap_or(
                                                workspace_height
                                                    * EMBEDDED_CHAT_DEFAULT_WORKSPACE_RATIO,
                                            );

                                        embedded_chat_resize_origin.set(Some((
                                            point.y,
                                            clamp_embedded_chat_height(
                                                start_height,
                                                workspace_height,
                                            ),
                                            workspace_height,
                                        )));
                                    });
                                },
                            }
                        }
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
                                            on_send_message.call(());
                                        }
                                    },
                                }
                                button {
                                    r#type: "button",
                                    disabled: draft().trim().is_empty() || is_sending(),
                                    class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-blue-500 text-white transition hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-45",
                                    "aria-label": "Отправить сообщение",
                                    onclick: move |_| on_send_message.call(()),
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
                    }
                }
            }
        }
    }
}
