//! Room text chat panel component.

use std::rc::Rc;

use cheenhub_contracts::realtime::TextChatMessage;
use dioxus::prelude::dioxus_elements::geometry::PixelsVector2D;
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::app::components::app_shell::ActiveRoom;
use crate::features::realtime::RealtimeHandle;

use super::realtime;

const BOTTOM_SCROLL_THRESHOLD: f64 = 24.0;
const OLDER_PAGE_SCROLL_THRESHOLD: f64 = 48.0;

#[derive(Clone, Copy)]
enum ScrollCommand {
    Bottom,
    SmoothBottom,
    Preserve { offset_y: f64, height: f64 },
}

#[derive(Clone)]
struct HistoryTarget {
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
}

#[derive(Clone, Copy)]
struct HistoryState {
    messages: Signal<Vec<TextChatMessage>>,
    has_more: Signal<bool>,
    initial_loading: Signal<bool>,
    history_error: Signal<Option<String>>,
    older_loading: Signal<bool>,
    older_error: Signal<Option<String>>,
    list_element: Signal<Option<Rc<MountedData>>>,
    pending_scroll: Signal<Option<ScrollCommand>>,
}

#[derive(Clone, Copy)]
struct ComposeState {
    draft: Signal<String>,
    messages: Signal<Vec<TextChatMessage>>,
    status: Signal<String>,
    is_sending: Signal<bool>,
    is_near_bottom: Signal<bool>,
    pending_scroll: Signal<Option<ScrollCommand>>,
}

/// Renders a realtime text chat panel for one room.
#[component]
pub(crate) fn ChatRoomPanel(server_id: String, room: ActiveRoom, compact: bool) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let mut messages = use_signal(Vec::<TextChatMessage>::new);
    let mut draft = use_signal(String::new);
    let status = use_signal(String::new);
    let is_sending = use_signal(|| false);
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

    use_hook(move || {
        load_initial_history(history_target, history_state);
    });

    use_hook(move || {
        let realtime = event_realtime.clone();
        spawn(async move {
            let mut receiver = realtime::subscribe_text_chat(&realtime);
            while let Some(message) = receiver.next().await {
                if message.room_id == event_room_id
                    && append_message(&mut messages, message)
                    && is_near_bottom()
                {
                    pending_scroll.set(Some(ScrollCommand::Bottom));
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

    let can_send = !is_sending() && !draft().trim().is_empty();
    let submit_message = use_callback(move |_| {
        if !is_sending() && !draft().trim().is_empty() {
            send_current_message(
                send_realtime.clone(),
                send_server_id.clone(),
                send_room_id.clone(),
                compose_state,
            );
        }
    });
    let load_older = use_callback(move |_| {
        load_older_history(older_target.clone(), history_state);
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

fn load_initial_history(target: HistoryTarget, mut state: HistoryState) {
    state.initial_loading.set(true);
    state.history_error.set(None);
    spawn(async move {
        match realtime::load_room_history(&target.realtime, target.server_id, target.room_id, None)
            .await
        {
            Ok(history) => {
                state.messages.set(history.messages);
                state.has_more.set(history.has_more);
                state.pending_scroll.set(Some(ScrollCommand::Bottom));
            }
            Err(error) => state.history_error.set(Some(error.to_string())),
        }
        state.initial_loading.set(false);
    });
}

fn load_older_history(target: HistoryTarget, mut state: HistoryState) {
    if (state.older_loading)() || !(state.has_more)() {
        return;
    }
    let Some(before_message_id) = (state.messages)().first().map(|message| message.id.clone())
    else {
        return;
    };

    state.older_loading.set(true);
    state.older_error.set(None);
    spawn(async move {
        let before_scroll = match state.list_element.cloned() {
            Some(element) => capture_scroll_position(element).await,
            None => None,
        };

        match realtime::load_room_history(
            &target.realtime,
            target.server_id,
            target.room_id,
            Some(before_message_id),
        )
        .await
        {
            Ok(history) => {
                prepend_messages(&mut state.messages, history.messages);
                state.has_more.set(history.has_more);
                if let Some((offset_y, height)) = before_scroll {
                    state
                        .pending_scroll
                        .set(Some(ScrollCommand::Preserve { offset_y, height }));
                }
            }
            Err(error) => state.older_error.set(Some(error.to_string())),
        }
        state.older_loading.set(false);
    });
}

async fn update_scroll_state(
    element: Rc<MountedData>,
    mut is_near_bottom: Signal<bool>,
    has_more: Signal<bool>,
    older_loading: Signal<bool>,
    initial_loading: Signal<bool>,
    load_older: Callback,
) {
    let Ok(offset) = element.get_scroll_offset().await else {
        return;
    };
    let Ok(scroll_size) = element.get_scroll_size().await else {
        return;
    };
    let Ok(rect) = element.get_client_rect().await else {
        return;
    };
    let bottom_gap = scroll_size.height - rect.size.height - offset.y;

    is_near_bottom.set(bottom_gap <= BOTTOM_SCROLL_THRESHOLD);
    if offset.y <= OLDER_PAGE_SCROLL_THRESHOLD
        && has_more()
        && !older_loading()
        && !initial_loading()
    {
        load_older.call(());
    }
}

async fn capture_scroll_position(element: Rc<MountedData>) -> Option<(f64, f64)> {
    let offset = element.get_scroll_offset().await.ok()?;
    let scroll_size = element.get_scroll_size().await.ok()?;

    Some((offset.y, scroll_size.height))
}

async fn apply_scroll_command(element: Rc<MountedData>, command: ScrollCommand) {
    match command {
        ScrollCommand::Bottom => {
            let Ok(scroll_size) = element.get_scroll_size().await else {
                return;
            };
            let _ = element
                .scroll(
                    PixelsVector2D::new(0.0, scroll_size.height),
                    ScrollBehavior::Instant,
                )
                .await;
        }
        ScrollCommand::SmoothBottom => {
            let Ok(scroll_size) = element.get_scroll_size().await else {
                return;
            };
            let _ = element
                .scroll(
                    PixelsVector2D::new(0.0, scroll_size.height),
                    ScrollBehavior::Smooth,
                )
                .await;
        }
        ScrollCommand::Preserve { offset_y, height } => {
            let Ok(scroll_size) = element.get_scroll_size().await else {
                return;
            };
            let next_offset = offset_y + (scroll_size.height - height);
            let _ = element
                .scroll(
                    PixelsVector2D::new(0.0, next_offset.max(0.0)),
                    ScrollBehavior::Instant,
                )
                .await;
        }
    }
}

fn send_current_message(
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    mut state: ComposeState,
) {
    let body = (state.draft)().trim().to_owned();
    if body.is_empty() {
        return;
    }
    state.draft.set(String::new());
    state.status.set(String::new());
    state.is_sending.set(true);

    spawn(async move {
        match realtime::send_text_message(&realtime, server_id, room_id, body).await {
            Ok(accepted) => {
                if append_message(&mut state.messages, accepted.message) && (state.is_near_bottom)()
                {
                    state.pending_scroll.set(Some(ScrollCommand::Bottom));
                }
            }
            Err(error) => state.status.set(error.to_string()),
        }
        state.is_sending.set(false);
    });
}

fn append_message(messages: &mut Signal<Vec<TextChatMessage>>, message: TextChatMessage) -> bool {
    let mut next_messages = messages();
    if next_messages
        .iter()
        .any(|saved_message| saved_message.id == message.id)
    {
        return false;
    }
    next_messages.push(message);
    messages.set(next_messages);

    true
}

fn prepend_messages(messages: &mut Signal<Vec<TextChatMessage>>, incoming: Vec<TextChatMessage>) {
    let saved_messages = messages();
    let mut next_messages = incoming
        .into_iter()
        .filter(|message| {
            !saved_messages
                .iter()
                .any(|saved_message| saved_message.id == message.id)
        })
        .collect::<Vec<_>>();

    next_messages.extend(saved_messages);
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
