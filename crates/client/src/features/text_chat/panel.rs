//! Room text chat panel component.

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::components::app_shell::ActiveRoom;
use crate::features::realtime::RealtimeHandle;

use super::realtime;

/// Renders a realtime text chat panel for one room.
#[component]
pub(crate) fn ChatRoomPanel(server_id: String, room: ActiveRoom, compact: bool) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let mut messages = use_signal(Vec::<TextChatMessage>::new);
    let mut draft = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut is_sending = use_signal(|| false);
    let event_room_id = room.id.clone();
    let history_server_id = server_id.clone();
    let history_room_id = room.id.clone();
    let send_server_id = server_id.clone();
    let send_room_id = room.id.clone();
    let history_realtime = realtime.clone();
    let event_realtime = realtime.clone();
    let send_realtime = realtime.clone();
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
    let history_resource = use_resource(move || {
        let realtime = history_realtime.clone();
        let server_id = history_server_id.clone();
        let room_id = history_room_id.clone();

        async move { realtime::load_room_history(&realtime, server_id, room_id).await }
    });
    let history_result = history_resource.read().clone();
    let is_loading = history_result.is_none() && messages().is_empty();

    use_effect(move || {
        let Some(Ok(history)) = history_resource.read().clone() else {
            return;
        };
        messages.set(history.messages);
        status.set(String::new());
    });

    use_hook(move || {
        let realtime = event_realtime.clone();
        spawn(async move {
            let mut receiver = realtime::subscribe_text_chat(&realtime);
            while let Some(message) = receiver.next().await {
                if message.room_id == event_room_id {
                    append_message(&mut messages, message);
                }
            }
        });
    });

    let can_send = !is_sending() && !draft().trim().is_empty();

    rsx! {
        div { class: "flex h-full min-h-0 flex-col",
            div { class: list_class,
                div { class: inner_class,
                    if is_loading {
                        div { class: "space-y-3",
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/80" }
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/60" }
                            div { class: "h-14 animate-pulse rounded-2xl bg-zinc-900/40" }
                        }
                    } else if let Some(Err(error)) = history_result {
                        div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                            "{error}"
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
                            div { key: "{message.id}", class: "flex gap-3",
                                div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100",
                                    "{initial(&message.author_nickname)}"
                                }
                                div { class: "min-w-0 flex-1",
                                    div { class: "mb-1 flex items-center gap-2",
                                        span { class: "truncate text-[12px] font-semibold text-zinc-100", "{message.author_nickname}" }
                                        span { class: "shrink-0 text-[10px] text-zinc-600", "{message_time(&message.created_at)}" }
                                    }
                                    div { class: "message-bubble whitespace-pre-wrap break-words rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]",
                                        "{message.body}"
                                    }
                                }
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
                    textarea {
                        rows: "1",
                        value: "{draft()}",
                        placeholder: "Сообщение в {placeholder_prefix} {room.name}",
                        class: "max-h-28 min-h-10 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600",
                        oninput: move |event| draft.set(event.value()),
                    }
                    button {
                        r#type: "button",
                        disabled: !can_send,
                        class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-accent text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:translate-y-0 disabled:hover:bg-accent",
                        "aria-label": "Отправить сообщение",
                        onclick: move |_| {
                            if can_send {
                                send_current_message(
                                    send_realtime.clone(),
                                    send_server_id.clone(),
                                    send_room_id.clone(),
                                    &mut draft,
                                    &mut messages,
                                    &mut status,
                                    &mut is_sending,
                                );
                            }
                        },
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" }
                        }
                    }
                }
            }
        }
    }
}

fn send_current_message(
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    draft: &mut Signal<String>,
    messages: &mut Signal<Vec<TextChatMessage>>,
    status: &mut Signal<String>,
    is_sending: &mut Signal<bool>,
) {
    let body = draft().trim().to_owned();
    if body.is_empty() {
        return;
    }
    draft.set(String::new());
    status.set(String::new());
    is_sending.set(true);

    let mut messages = *messages;
    let mut status = *status;
    let mut is_sending = *is_sending;
    spawn(async move {
        match realtime::send_text_message(&realtime, server_id, room_id, body).await {
            Ok(accepted) => append_message(&mut messages, accepted.message),
            Err(error) => status.set(error.to_string()),
        }
        is_sending.set(false);
    });
}

fn append_message(messages: &mut Signal<Vec<TextChatMessage>>, message: TextChatMessage) {
    let mut next_messages = messages();
    if next_messages
        .iter()
        .any(|saved_message| saved_message.id == message.id)
    {
        return;
    }
    next_messages.push(message);
    messages.set(next_messages);
}

fn initial(nickname: &str) -> String {
    nickname
        .chars()
        .next()
        .map(|letter| letter.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_owned())
}

fn message_time(created_at: &str) -> String {
    created_at
        .split('T')
        .nth(1)
        .and_then(|time| time.get(0..5))
        .unwrap_or("")
        .to_owned()
}
