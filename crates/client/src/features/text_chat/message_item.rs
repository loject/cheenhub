//! Компонент элемента сообщения текстового чата.

use cheenhub_contracts::{realtime::TextChatMessage, rest::DmMessageDeliveryStatus};
use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;

use super::message_edit_dialog::{InlineMessageEditState, RoomMessageInlineEditor};

/// Рендерит одну строку сообщения текстового чата.
#[component]
pub(crate) fn ChatMessageItem(
    message: TextChatMessage,
    #[props(default)] anchor_elements: Option<super::scroll_anchor_runtime::AnchorElements>,
    animate: bool,
    removing: bool,
    #[props(default)] show_hover_time: bool,
    #[props(default)] allow_own_delete: bool,
    can_delete_messages: bool,
    on_delete: EventHandler<String>,
    #[props(default)] on_edit: Option<EventHandler<String>>,
    #[props(default)] inline_edit: Option<InlineMessageEditState>,
    #[props(default)] external_attachment: bool,
    #[props(default)] max_edit_chars: Option<usize>,
    #[props(default)] can_pin_messages: bool,
    #[props(default)] is_pinned: bool,
    #[props(default)] on_pin_toggle: Option<EventHandler<String>>,
    #[props(default)] highlighted: bool,
    children: Element,
) -> Element {
    let register_anchor =
        super::scroll_anchor_runtime::use_message_anchor(message.id.clone(), anchor_elements);
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own = message.author_user_id == current_user.id;
    let can_delete = can_delete_messages || (is_own && allow_own_delete);
    let can_edit = is_own && on_edit.is_some();
    let active_edit = inline_edit.filter(|state| can_edit && state.message_id == message.id);
    let is_editing = active_edit.is_some();
    let can_pin = can_pin_messages && on_pin_toggle.is_some();
    let mut menu_pos = use_signal(|| None::<(f64, f64)>);
    let highlight_class = if is_editing {
        "bg-white/[0.025]"
    } else if highlighted {
        "bg-blue-400/15 ring-1 ring-inset ring-blue-400/60"
    } else if is_pinned {
        "bg-blue-500/[0.045] hover:bg-blue-500/[0.08]"
    } else {
        "hover:bg-white/[0.03]"
    };
    let row_class = match (animate, removing) {
        (_, true) => "chat-message-removing",
        (true, false) => "chat-message",
        (false, false) => "",
    };
    let content_class = if is_own {
        "w-full whitespace-pre-wrap break-words text-left text-[14.5px] leading-[22.5px] tracking-[0.002em] text-[#e2ecfd]"
    } else {
        "w-full whitespace-pre-wrap break-words text-left text-[14.5px] leading-[22.5px] tracking-[0.002em] text-[#d4d8e2]"
    };
    let item_time = message_time(&message.created_at);
    let item_datetime = super::message_date::full_message_datetime(&message.created_at);
    let quick_pin_handler = on_pin_toggle.clone();
    let quick_pin_id = message.id.clone();
    let quick_delete_id = message.id.clone();
    let quick_edit_handler = on_edit.clone();
    let quick_edit_id = message.id.clone();
    let edited_datetime = message
        .edited_at
        .as_deref()
        .map(super::message_date::full_message_datetime);

    rsx! {
        div {
            class: "{row_class} {highlight_class} group/msg relative flex w-full items-start justify-between gap-3 rounded-xl px-2 py-1 transition-colors duration-100 hover:z-30 focus-within:z-30",
            onmounted: move |event| register_anchor.call(event.data.clone()),
            oncontextmenu: move |event| {
                if is_editing || (!can_delete && !can_pin && !can_edit) {
                    return;
                }
                event.prevent_default();
                event.stop_propagation();
                let p = event.client_coordinates();
                menu_pos.set(Some((p.x, p.y)));
            },
            if is_own || is_pinned {
                span {
                    class: if is_pinned { "pointer-events-none absolute bottom-1.5 left-0 top-1.5 w-[2.5px] rounded-full bg-blue-400" } else { "pointer-events-none absolute bottom-1.5 left-0 top-1.5 w-[2.5px] rounded-full bg-blue-400/35" },
                    "aria-hidden": "true",
                }
            }
            if show_hover_time {
                time {
                    class: "pointer-events-none absolute -left-[3.1rem] top-2 hidden w-10 select-none text-right text-[10px] font-medium tabular-nums text-zinc-400 opacity-0 transition-opacity duration-100 group-hover/msg:opacity-100 group-focus-within/msg:opacity-100 min-[901px]:block",
                    datetime: "{message.created_at}",
                    title: "{item_datetime}",
                    "{item_time}"
                }
            }
            div { class: "flex min-w-0 max-w-[46rem] flex-1 flex-col items-start gap-1.5 pl-0.5",
                if is_pinned {
                    div { class: "inline-flex items-center gap-1 text-[11px] font-medium text-blue-300/90",
                        svg { class: "h-3 w-3", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16 3 5 5-4 1-3 4-1 3-2 2-5-5 2-2 3-1 4-3 1-4zM9 15l-5 6" }
                        }
                        span { "Закреплено в комнате" }
                    }
                }
                if let Some(edit_state) = active_edit {
                    RoomMessageInlineEditor {
                        key: "{message.id}",
                        message: message.clone(),
                        external_attachment,
                        max_edit_chars,
                        saving: edit_state.saving,
                        error: edit_state.error,
                        on_cancel: edit_state.on_cancel,
                        on_save: edit_state.on_save,
                    }
                } else if !message.body.is_empty() {
                    div { class: content_class,
                        span { class: "[overflow-wrap:anywhere]", "{message.body}" }
                        if is_own {
                            if let Some(status) = message.delivery_status {
                                {delivery_status_marks(status)}
                            }
                        }
                    }
                }
                {children}
                if !is_editing {
                    if let Some(edited_datetime) = &edited_datetime {
                        span {
                            class: "text-[10px] font-medium text-zinc-500",
                            title: "Изменено: {edited_datetime}",
                            "(изменено)"
                        }
                    }
                }
            }
            if is_own && message.body.is_empty() && !is_editing {
                if let Some(status) = message.delivery_status {
                    {delivery_status_marks(status)}
                }
            }
            if (can_pin || can_delete || can_edit) && !is_editing {
                div {
                    class: "pointer-events-none absolute -top-3 right-2 z-20 flex items-center gap-0.5 rounded-xl border border-zinc-800 bg-zinc-950/95 p-1 opacity-0 shadow-[0_8px_24px_rgba(0,0,0,0.45)] backdrop-blur-md transition-opacity duration-100 group-hover/msg:pointer-events-auto group-hover/msg:opacity-100 group-focus-within/msg:pointer-events-auto group-focus-within/msg:opacity-100",
                    if show_hover_time {
                        span { class: "px-1.5 text-[10px] tabular-nums text-zinc-400 min-[901px]:hidden", "{item_time}" }
                    }
                    if can_pin {
                        if let Some(handler) = quick_pin_handler {
                            button {
                                r#type: "button",
                                class: if is_pinned { "flex h-7 items-center gap-1 rounded-lg bg-blue-500/20 px-2 text-[11px] font-medium text-blue-200 transition-colors hover:bg-blue-500/30" } else { "flex h-7 items-center gap-1 rounded-lg px-2 text-[11px] font-medium text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-zinc-100" },
                                title: if is_pinned { "Открепить сообщение" } else { "Закрепить сообщение" },
                                onclick: move |_| handler.call(quick_pin_id.clone()),
                                svg { class: "h-3.5 w-3.5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.9",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16 3 5 5-4 1-3 4-1 3-2 2-5-5 2-2 3-1 4-3 1-4zM9 15l-5 6" }
                                }
                                if is_pinned { "Открепить" } else { "Закрепить" }
                            }
                        }
                    }
                    if can_edit {
                        if let Some(handler) = quick_edit_handler {
                            button {
                                r#type: "button",
                                class: "flex h-7 items-center gap-1 rounded-lg px-2 text-[11px] font-medium text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-white",
                                title: "Редактировать сообщение",
                                onclick: move |_| handler.call(quick_edit_id.clone()),
                                svg { class: "h-3.5 w-3.5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.9",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16.9 3.9 3.2 3.2M4 20l4.5-1 11-11a2.3 2.3 0 0 0-3.2-3.2l-11 11L4 20z" }
                                }
                                "Изменить"
                            }
                        }
                    }
                    if can_delete {
                        button {
                            r#type: "button",
                            class: "flex h-7 w-7 items-center justify-center rounded-lg text-zinc-400 transition-colors hover:bg-red-500/15 hover:text-red-300",
                            title: "Удалить сообщение",
                            "aria-label": "Удалить сообщение",
                            onclick: move |_| on_delete.call(quick_delete_id.clone()),
                            svg { class: "h-3.5 w-3.5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.9",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 6h18M8 6V4h8v2m3 0-1 14H6L5 6m5 4v7m4-7v7" }
                            }
                        }
                    }
                }
            }
        }

        if let Some((x, y)) = menu_pos() {
            {
                let message_id = message.id.clone();
                let pin_message_id = message.id.clone();
                let edit_message_id = message.id.clone();
                rsx! {
                    div {
                        class: "fixed inset-0 z-[999]",
                        onclick: move |_| menu_pos.set(None),
                    }
                    div {
                        class: "fixed z-[1000] min-w-[180px] overflow-hidden rounded-[16px] border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_20px_60px_rgba(0,0,0,.55)] backdrop-blur-xl",
                        style: "left: clamp(12px, {x}px, calc(100vw - 200px)); top: clamp(12px, {y}px, calc(100vh - 80px));",
                        onclick: move |event| event.stop_propagation(),
                        if can_edit {
                            if let Some(handler) = on_edit {
                                button {
                                    r#type: "button",
                                    class: "flex min-h-10 w-full items-center gap-2.5 rounded-xl px-3 py-2.5 text-left text-[13px] text-zinc-200 transition-colors hover:bg-white/10",
                                    onclick: move |_| {
                                        menu_pos.set(None);
                                        handler.call(edit_message_id.clone());
                                    },
                                    "Редактировать сообщение"
                                }
                            }
                        }
                        if can_pin {
                            if let Some(on_pin_toggle) = on_pin_toggle {
                                button {
                                    r#type: "button",
                                    class: "flex min-h-10 w-full items-center gap-2.5 rounded-xl px-3 py-2.5 text-left text-[13px] text-blue-200 transition-colors hover:bg-blue-400/10",
                                    onclick: move |_| {
                                        menu_pos.set(None);
                                        on_pin_toggle.call(pin_message_id.clone());
                                    },
                                    if is_pinned { "Открепить сообщение" } else { "Закрепить сообщение" }
                                }
                            }
                        }
                        if can_delete {
                        button {
                            r#type: "button",
                            class: "flex min-h-10 w-full items-center gap-2.5 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background-color,color,transform] duration-150 hover:bg-red-500/10 hover:text-red-200 active:scale-[0.96]",
                            onclick: move |_| {
                                menu_pos.set(None);
                                on_delete.call(message_id.clone());
                            },
                            svg {
                                class: "h-4 w-4 shrink-0",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "1.9",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    d: "m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0",
                                }
                            }
                            "Удалить сообщение"
                        }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn message_time(created_at: &str) -> String {
    created_at
        .split('T')
        .nth(1)
        .and_then(|time| time.get(0..5))
        .unwrap_or("")
        .to_owned()
}

fn delivery_status_title(status: DmMessageDeliveryStatus) -> &'static str {
    match status {
        DmMessageDeliveryStatus::Accepted => "Сообщение отправлено",
        DmMessageDeliveryStatus::Read => "Сообщение прочитано",
    }
}

fn delivery_status_marks(status: DmMessageDeliveryStatus) -> Element {
    rsx! {
        span {
            class: "mb-[1px] inline-flex shrink-0 items-center text-[10px] font-semibold leading-none text-blue-300/50",
            title: "{delivery_status_title(status)}",
            match status {
                DmMessageDeliveryStatus::Accepted => rsx! {
                    span { "✓" }
                },
                DmMessageDeliveryStatus::Read => rsx! {
                    span { class: "inline-flex -space-x-[3px]",
                        span { "✓" }
                        span { "✓" }
                    }
                },
            }
        }
    }
}
