//! Room text chat panel component.

use std::rc::Rc;
use std::time::Duration;

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_timers::future::sleep;

use crate::features::app::components::app_shell::ActiveRoom;
use crate::features::realtime::RealtimeHandle;

use super::compose::{ComposeState, send_current_message};
use super::history::{
    HistoryState, HistoryTarget, load_initial_history, load_initial_history_when_connected,
    load_older_history,
};
use super::message_item::ChatMessageItem;
use super::messages::{append_message, is_appearing_message, remove_message};
use super::realtime::{self, TextChatEvent};
use super::scroll::{ScrollCommand, apply_scroll_command, update_scroll_state};

/// Renders a realtime text chat panel for one room.
#[component]
pub(crate) fn ChatRoomPanel(server_id: String, room: ActiveRoom, compact: bool) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let mut messages = use_signal(Vec::<TextChatMessage>::new);
    let mut appearing_message_ids = use_signal(Vec::<String>::new);
    let mut removing_message_ids = use_signal(Vec::<String>::new);
    let mut draft = use_signal(String::new);
    let mut status = use_signal(String::new);
    let is_sending = use_signal(|| false);
    let mut is_uploading_image = use_signal(|| false);
    let initial_loading = use_signal(|| true);
    let older_loading = use_signal(|| false);
    let history_error = use_signal(|| None::<String>);
    let older_error = use_signal(|| None::<String>);
    let has_more = use_signal(|| false);
    let is_near_bottom = use_signal(|| true);
    let mut list_element = use_signal(|| None::<Rc<MountedData>>);
    let mut pending_scroll = use_signal(|| None::<ScrollCommand>);
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
    let send_realtime = realtime.clone();
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
    let compose_state = ComposeState {
        draft,
        messages,
        appearing_message_ids,
        status,
        is_sending,
        is_near_bottom,
        pending_scroll,
    };
    let placeholder_prefix = if compact { "&" } else { "#" };
    let list_class = if compact {
        "min-h-0 flex-1 overflow-y-auto p-4 pt-2"
    } else {
        "min-h-0 flex-1 overflow-y-auto p-5 lg:p-6"
    };
    let inner_class = if compact {
        "space-y-4"
    } else {
        "mx-auto flex w-full max-w-3xl flex-col gap-4"
    };
    let input_outer_class = if compact {
        "shrink-0 border-t border-zinc-800/80 bg-zinc-950/35 p-3"
    } else {
        "shrink-0 border-t border-zinc-800/80 bg-zinc-950/55 p-4 backdrop-blur-xl"
    };
    let input_wrap_class = if compact {
        "chat-input-wrap flex items-end gap-2 rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]"
    } else {
        "chat-input-wrap mx-auto flex max-w-3xl items-end gap-2 rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]"
    };
    let appearing_message_ids_list = appearing_message_ids();
    let removing_message_ids_list = removing_message_ids();

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
                                sleep(Duration::from_millis(220)).await;
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

    let can_send = !is_sending() && !is_uploading_image() && !draft().trim().is_empty();
    let submit_realtime = send_realtime.clone();
    let submit_server_id = send_server_id.clone();
    let submit_room_id = send_room_id.clone();
    let submit_message = use_callback(move |_| {
        if !is_sending() && !draft().trim().is_empty() {
            send_current_message(
                submit_realtime.clone(),
                submit_server_id.clone(),
                submit_room_id.clone(),
                compose_state,
            );
        }
    });
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
            sleep(Duration::from_millis(220)).await;
            remove_message(&mut messages, &message_id);
            removing_message_ids.write().retain(|id| id != &message_id);
        });
    });
    let upload_image = use_callback(move |event: Event<FormData>| {
        if is_uploading_image() {
            return;
        }
        let Some(file) = event.files().into_iter().next() else {
            return;
        };
        if file.size() > 10 * 1024 * 1024 {
            status.set("Изображение слишком большое. Загрузи файл до 10 МБ.".to_owned());
            return;
        }

        let realtime = send_realtime.clone();
        let server_id = send_server_id.clone();
        let room_id = send_room_id.clone();
        let original_filename = Some(file.name());
        is_uploading_image.set(true);
        status.set(String::new());
        info!(
            file_name = original_filename.as_deref().unwrap_or(""),
            file_size = file.size(),
            "uploading text chat image over realtime"
        );
        spawn(async move {
            let result = match file.read_bytes().await {
                Ok(bytes) => {
                    match realtime::upload_chat_image(
                        &realtime,
                        server_id.clone(),
                        room_id.clone(),
                        original_filename,
                        bytes.to_vec(),
                    )
                    .await
                    {
                        Ok(uploaded) => {
                            realtime::send_image_message(&realtime, server_id, room_id, uploaded.id)
                                .await
                        }
                        Err(error) => Err(error),
                    }
                }
                Err(error) => {
                    warn!(?error, "failed to read selected text chat image");
                    Err(crate::features::realtime::RealtimeError::new(
                        "Не удалось прочитать выбранный файл.",
                    ))
                }
            };

            match result {
                Ok(accepted) => {
                    if append_message(&mut messages, &mut appearing_message_ids, accepted.message)
                        && is_near_bottom()
                    {
                        pending_scroll.set(Some(ScrollCommand::Bottom));
                    }
                }
                Err(error) => {
                    warn!(%error, "text chat image send failed");
                    status.set(error.to_string());
                }
            }
            is_uploading_image.set(false);
        });
    });

    rsx! {
        div { class: "flex h-full min-h-0 flex-col",
            div {
                class: list_class,
                onmounted: move |event| list_element.set(Some(event.data.clone())),
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
                    if older_loading() {
                        div { class: "flex justify-center py-2",
                            div { class: "h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400" }
                        }
                    } else if let Some(error) = older_error() {
                        div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-center text-[12px] leading-5 text-red-200",
                            p { "{error}" }
                            button {
                                r#type: "button",
                                class: "mt-2 rounded-lg border border-red-300/20 px-3 py-1 text-[12px] font-medium text-red-100 transition-colors hover:border-red-200/40 hover:bg-red-400/10",
                                onclick: move |_| load_older.call(()),
                                "Повторить"
                            }
                        }
                    }
                    if initial_loading() && messages().is_empty() {
                        div { class: "space-y-3",
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/80" }
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/60" }
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/40" }
                        }
                    } else if let Some(error) = history_error() {
                        div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-3 text-center text-[12px] leading-5 text-red-200",
                            p { "{error}" }
                            button {
                                r#type: "button",
                                class: "mt-2 rounded-lg border border-red-300/20 px-3 py-1 text-[12px] font-medium text-red-100 transition-colors hover:border-red-200/40 hover:bg-red-400/10",
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
                    } else if messages().is_empty() {
                        div { class: "rounded-[20px] border border-zinc-800 bg-zinc-900/60 p-6 text-center",
                            p { class: "text-[13px] font-medium text-zinc-100", "Сообщений пока нет" }
                            p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                                "Напиши первое сообщение в этой комнате."
                            }
                        }
                    } else {
                        for message in messages() {
                            ChatMessageItem {
                                key: "{message.id}",
                                animate: is_appearing_message(
                                    &message.id,
                                    &appearing_message_ids_list,
                                ),
                                removing: removing_message_ids_list.contains(&message.id),
                                message: message.clone(),
                                on_delete: move |id| on_delete_message.call(id),
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
            if !status().is_empty() {
                p { class: "mx-auto w-full max-w-3xl px-4 pb-2 text-[11px] leading-4 text-red-200",
                    "{status()}"
                }
            }
            div { class: input_outer_class,
                div { class: input_wrap_class,
                    label {
                        class: "flex h-10 w-10 shrink-0 cursor-pointer items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-white/15 hover:bg-zinc-800 hover:text-zinc-100 has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-45 has-[:disabled]:hover:translate-y-0",
                        title: "Прикрепить изображение",
                        input {
                            class: "sr-only",
                            r#type: "file",
                            accept: "image/png,image/jpeg,image/gif,image/webp,image/*",
                            disabled: is_sending() || is_uploading_image(),
                            onchange: move |event| upload_image.call(event),
                        }
                        if is_uploading_image() {
                            span { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-300" }
                        } else {
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m18.375 12.739-7.693 7.693a4.5 4.5 0 0 1-6.364-6.364l10.94-10.94a3 3 0 1 1 4.243 4.243L8.552 18.32a1.5 1.5 0 1 1-2.121-2.121l9.879-9.879" }
                            }
                        }
                    }
                    textarea {
                        rows: "1",
                        value: "{draft()}",
                        placeholder: "Сообщение в {placeholder_prefix} {room.name}",
                        class: "max-h-28 min-h-10 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600",
                        oninput: move |event| draft.set(event.value()),
                        onkeydown: move |event| {
                            if event.key() == Key::Enter && !event.modifiers().shift() {
                                event.prevent_default();
                                submit_message.call(());
                            }
                        },
                    }
                    button {
                        r#type: "button",
                        disabled: !can_send,
                        class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-accent text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:translate-y-0 disabled:hover:bg-accent",
                        "aria-label": "Отправить сообщение",
                        onclick: move |_| submit_message.call(()),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" }
                        }
                    }
                }
            }
        }
    }
}
