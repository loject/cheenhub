//! Компонент панели текстового чата комнаты.

use std::rc::Rc;
use std::time::Duration;

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::components::app_shell::ActiveRoom;
use crate::features::app::server_permissions::ServerPermissionsContext;
use crate::features::message_composer::{MessageComposeState, MessageComposer};
use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_duration;

use super::compose::use_room_message_operations;
use super::history::{
    HistoryState, HistoryTarget, load_initial_history, load_initial_history_when_connected,
    load_older_history,
};
use super::messages::{append_message, remove_message};
use super::realtime::{self, TextChatEvent};
use super::scroll::{ScrollCommand, apply_scroll_command, update_scroll_state};
use super::{
    CHAT_CONTENT_CLASS, ChatHistoryLoadingState, ChatMessageDateDivider, ChatMessageGroup,
    VirtualChatLayout, VirtualChatRow, prepare_text_chat_groups, use_history_overflow,
};

/// Рендерит панель realtime-текстового чата для одной комнаты.
#[component]
pub(crate) fn ChatRoomPanel(server_id: String, room: ActiveRoom, compact: bool) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let permissions = use_context::<ServerPermissionsContext>();
    let room_compose_state = use_context::<MessageComposeState>();
    let mut messages = use_signal(Vec::<TextChatMessage>::new);
    let mut appearing_message_ids = use_signal(Vec::<String>::new);
    let mut removing_message_ids = use_signal(Vec::<String>::new);
    let initial_loading = use_signal(|| true);
    let older_loading = use_signal(|| false);
    let history_error = use_signal(|| None::<String>);
    let older_error = use_signal(|| None::<String>);
    let has_more = use_signal(|| false);
    let is_near_bottom = use_signal(|| true);
    let mut list_element = use_signal(|| None::<Rc<MountedData>>);
    let (history_overflowing, measure_history) = use_history_overflow(list_element);
    let mut pending_scroll = use_signal(|| None::<ScrollCommand>);
    let virtual_layout = use_signal(VirtualChatLayout::default);
    let event_room_id = room.id.clone();
    let history_server_id = server_id.clone();
    let history_room_id = room.id.clone();
    let older_server_id = server_id.clone();
    let older_room_id = room.id.clone();
    let send_server_id = server_id.clone();
    let send_room_id = room.id.clone();
    let delete_server_id = server_id.clone();
    let delete_room_id = room.id.clone();
    let delete_realtime = realtime.clone();
    let history_realtime = realtime.clone();
    let event_realtime = realtime.clone();
    let older_realtime = realtime.clone();
    let history_target = HistoryTarget {
        realtime: history_realtime,
        server_id: history_server_id,
        room_id: history_room_id,
    };
    let older_target = HistoryTarget {
        realtime: older_realtime,
        server_id: older_server_id,
        room_id: older_room_id,
    };
    let history_state = HistoryState {
        messages,
        appearing_message_ids,
        has_more,
        initial_loading,
        history_error,
        older_loading,
        older_error,
        list_element,
        pending_scroll,
    };
    let placeholder_prefix = if compact { "&" } else { "#" };
    let list_class = if compact {
        "min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto bg-[#08090b] p-4 pt-3"
    } else {
        "min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto bg-[#08090b] px-5 py-7 lg:px-8 lg:py-8"
    };
    let inner_class = if compact {
        "min-w-0 w-full space-y-4"
    } else {
        CHAT_CONTENT_CLASS
    };
    let appearing_message_ids_list = appearing_message_ids();
    let removing_message_ids_list = removing_message_ids();
    let rendered_messages = messages();
    let has_messages = !rendered_messages.is_empty();
    let message_groups = prepare_text_chat_groups(&rendered_messages);
    let message_group_ids = message_groups
        .iter()
        .map(|(group_key, _, _, _)| group_key.clone())
        .collect::<Vec<_>>();
    let rendered_group_range = virtual_layout.read().rendered_range(&message_group_ids);

    use_hook(move || {
        load_initial_history_when_connected(history_target, history_state);
    });

    use_hook(move || {
        let realtime = event_realtime.clone();
        spawn(async move {
            let mut receiver = realtime::subscribe_text_chat(&realtime);
            while let Some(event) = receiver.next().await {
                match event {
                    TextChatEvent::MessageCreated(message) => {
                        if message.room_id == event_room_id
                            && append_message(&mut messages, &mut appearing_message_ids, message)
                            && is_near_bottom()
                        {
                            pending_scroll.set(Some(ScrollCommand::Bottom));
                        }
                    }
                    TextChatEvent::MessageDeleted(payload) => {
                        if payload.room_id == event_room_id {
                            let message_id = payload.message_id.clone();
                            removing_message_ids.write().push(message_id.clone());
                            spawn(async move {
                                sleep_duration(Duration::from_millis(220)).await;
                                remove_message(&mut messages, &message_id);
                                removing_message_ids.write().retain(|id| id != &message_id);
                            });
                        }
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

    let operations = use_room_message_operations(
        realtime.clone(),
        send_server_id,
        send_room_id,
        messages,
        appearing_message_ids,
        pending_scroll,
    );
    let load_older = use_callback(move |_| {
        load_older_history(older_target.clone(), history_state);
    });
    let on_delete_message = use_callback(move |message_id: String| {
        let realtime = delete_realtime.clone();
        let server_id = delete_server_id.clone();
        let room_id = delete_room_id.clone();
        removing_message_ids.write().push(message_id.clone());
        spawn(async move {
            let _ =
                realtime::delete_text_message(&realtime, server_id, room_id, message_id.clone())
                    .await;
            sleep_duration(Duration::from_millis(220)).await;
            remove_message(&mut messages, &message_id);
            removing_message_ids.write().retain(|id| id != &message_id);
        });
    });
    rsx! {
        div { class: "flex h-full min-h-0 min-w-0 w-full flex-col bg-[#08090b]",
            div {
                class: list_class,
                onmounted: move |event| {
                    list_element.set(Some(event.data.clone()));
                    measure_history.call(());
                },
                onresize: move |_| measure_history.call(()),
                onscroll: move |_| {
                    if let Some(element) = list_element.cloned() {
                        spawn(async move {
                            update_scroll_state(
                                element,
                                is_near_bottom,
                                has_more,
                                older_loading,
                                initial_loading,
                                load_older,
                            ).await;
                        });
                    }
                },
                div { class: inner_class,
                    onresize: move |_| measure_history.call(()),
                    if older_loading() {
                        div { class: "flex items-center justify-center gap-2 py-2 text-[11px] text-zinc-500", role: "status", "aria-live": "polite",
                            div { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-800 border-t-blue-400", "aria-hidden": "true" }
                            "Загружаем ранние сообщения…"
                        }
                    } else if let Some(error) = older_error() {
                        div { class: "rounded-[16px] bg-red-500/[0.08] px-4 py-3 text-center text-[12px] leading-5 text-red-200 shadow-[0_0_0_1px_rgba(248,113,113,0.16)]",
                            p { "{error}" }
                            button {
                                r#type: "button",
                                class: "mt-2 min-h-10 rounded-xl bg-red-400/10 px-4 text-[12px] font-medium text-red-100 transition-[background-color,color,transform] duration-150 hover:bg-red-400/15 hover:text-white active:scale-[0.96]",
                                onclick: move |_| load_older.call(()),
                                "Повторить"
                            }
                        }
                    }
                    if initial_loading() && !has_messages {
                        ChatHistoryLoadingState {}
                    } else if let Some(error) = history_error() {
                        div { class: "mx-auto max-w-md rounded-[18px] bg-red-500/[0.08] px-6 py-5 text-center text-[12px] leading-5 text-red-200 shadow-[0_0_0_1px_rgba(248,113,113,0.16)]",
                            p { class: "font-semibold text-red-100", "Не удалось загрузить сообщения" }
                            p { "{error}" }
                            button {
                                r#type: "button",
                                class: "mt-3 min-h-10 rounded-xl bg-red-400/10 px-4 text-[12px] font-medium text-red-100 transition-[background-color,color,transform] duration-150 hover:bg-red-400/15 hover:text-white active:scale-[0.96]",
                                onclick: move |_| {
                                    load_initial_history(
                                        HistoryTarget {
                                            realtime: realtime.clone(),
                                            server_id: server_id.clone(),
                                            room_id: room.id.clone(),
                                        },
                                        history_state,
                                    );
                                },
                                "Повторить"
                            }
                        }
                    } else if !has_messages {
                        div { class: "mx-auto flex max-w-md flex-col items-center px-6 py-12 text-center",
                            div { class: "mb-4 flex h-12 w-12 items-center justify-center rounded-[16px] bg-blue-500/10 text-[24px] font-light text-blue-400 shadow-[0_0_0_1px_rgba(96,165,250,0.16)]", "#" }
                            p { class: "text-[15px] font-semibold tracking-[-0.02em] text-zinc-100", "Здесь начнётся разговор" }
                            p { class: "mt-1.5 text-[12px] leading-5 text-zinc-500",
                                "Напиши первое сообщение в этой комнате."
                            }
                        }
                    } else {
                        for (group_index, (group_key, date_label, estimated_height, group)) in message_groups.iter().cloned().enumerate() {
                            div { key: "{group_key}", class: "contents",
                                if let Some(label) = date_label {
                                    ChatMessageDateDivider { label, overflowing: history_overflowing() }
                                }
                                VirtualChatRow {
                                    row_id: group_key.clone(),
                                    active: rendered_group_range.contains(&group_index),
                                    estimated_height,
                                    layout: virtual_layout,
                                    ChatMessageGroup {
                                        messages: group,
                                        appearing_message_ids: appearing_message_ids_list.clone(),
                                        removing_message_ids: removing_message_ids_list.clone(),
                                        can_delete_messages: permissions.can_delete_messages,
                                        on_delete: move |id| on_delete_message.call(id),
                                        server_id: server_id.clone(),
                                        room_id: room.id.clone(),
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "relative",
                if !is_near_bottom() && has_messages {
                    div { class: "pointer-events-none absolute bottom-3 right-4 z-20",
                    button {
                        r#type: "button",
                        class: "group pointer-events-auto relative flex h-10 w-10 items-center justify-center rounded-full bg-zinc-900/95 text-blue-200 shadow-[0_8px_22px_rgba(0,0,0,0.35),0_0_0_1px_rgba(255,255,255,0.08)] transition-[background-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-zinc-800 hover:text-blue-100 active:scale-[0.96]",
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
            if !room.can_write {
                div { class: "shrink-0 px-3 pb-3 pt-2",
                    {super::read_only_notice::read_only_notice()}
                }
            } else {
                MessageComposer {
                    state: room_compose_state,
                    operations,
                    placeholder: format!("Сообщение в {placeholder_prefix} {}", room.name),
                    compact,
                }
            }
        }
    }
}
