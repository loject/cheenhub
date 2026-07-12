//! Компонент элемента сообщения текстового чата.

use cheenhub_contracts::{realtime::TextChatMessage, rest::DmMessageDeliveryStatus};
use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;

use super::image_attachment::ChatImageAttachment;
use super::message_date::full_message_datetime;

/// Рендерит одну строку сообщения текстового чата.
#[component]
pub(crate) fn ChatMessageItem(
    message: TextChatMessage,
    animate: bool,
    removing: bool,
    can_delete_messages: bool,
    on_delete: EventHandler<String>,
) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let is_own = message.author_user_id == current_user.id;
    let can_delete = is_own || can_delete_messages;
    let mut menu_pos = use_signal(|| None::<(f64, f64)>);

    let outer_class = match (animate, removing) {
        (_, true) => "chat-message-removing flex gap-3",
        (true, false) => "chat-message flex gap-3",
        (false, false) => "flex gap-3",
    };
    let bubble_class = if is_own {
        "message-bubble flex items-end gap-2 whitespace-pre-wrap break-words rounded-[20px] border border-blue-500/20 bg-blue-500/10 px-3 py-2 text-[13px] leading-5 text-blue-100 transition-[border-color,background] duration-200 hover:border-blue-400/40 hover:bg-blue-500/15"
    } else {
        "message-bubble flex items-end gap-2 whitespace-pre-wrap break-words rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]"
    };
    let time_class = if is_own {
        "mb-[1px] shrink-0 text-[10px] leading-none text-blue-300/70"
    } else {
        "mb-[1px] shrink-0 text-[10px] leading-none text-zinc-500"
    };
    let sent_time = message_time(&message.created_at);
    let sent_datetime = full_message_datetime(&message.created_at);

    rsx! {
        div {
            class: outer_class,
            oncontextmenu: move |event| {
                if !can_delete {
                    return;
                }
                event.prevent_default();
                event.stop_propagation();
                let p = event.client_coordinates();
                menu_pos.set(Some((p.x, p.y)));
            },
            div { class: "min-w-0 flex-1",
                if !message.body.is_empty() {
                    div { class: bubble_class,
                        span { class: "min-w-0 flex-1", "{message.body}" }
                        span { class: "group/message-time relative inline-flex shrink-0",
                            span { class: time_class, "{sent_time}" }
                            span {
                                role: "tooltip",
                                class: "pointer-events-none absolute bottom-[calc(100%+8px)] right-0 z-30 w-max max-w-[min(18rem,calc(100vw-2rem))] rounded-lg border border-zinc-800 bg-zinc-950/95 px-2.5 py-1.5 text-[11px] font-medium leading-4 text-zinc-200 opacity-0 shadow-[0_8px_22px_rgba(0,0,0,0.35)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover/message-time:opacity-100 group-focus-within/message-time:opacity-100",
                                "Отправлено {sent_datetime}"
                            }
                        }
                        if is_own {
                            if let Some(status) = message.delivery_status {
                                {delivery_status_marks(status)}
                            }
                        }
                    }
                }
                for attachment in message.attachments.iter().cloned() {
                    ChatImageAttachment {
                        key: "{attachment.id}",
                        attachment,
                    }
                }
            }
        }

        if let Some((x, y)) = menu_pos() {
            {
                let message_id = message.id.clone();
                rsx! {
                    div {
                        class: "fixed inset-0 z-[999]",
                        onclick: move |_| menu_pos.set(None),
                    }
                    div {
                        class: "fixed z-[1000] min-w-[180px] overflow-hidden rounded-[16px] border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_20px_60px_rgba(0,0,0,.55)] backdrop-blur-xl",
                        style: "left: clamp(12px, {x}px, calc(100vw - 200px)); top: clamp(12px, {y}px, calc(100vh - 80px));",
                        onclick: move |event| event.stop_propagation(),
                        button {
                            r#type: "button",
                            class: "flex w-full items-center gap-2.5 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background,color] duration-150 hover:bg-red-500/10 hover:text-red-200",
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

pub(super) fn message_time(created_at: &str) -> String {
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
            class: "mb-[1px] inline-flex shrink-0 items-center text-[10px] font-semibold leading-none text-blue-300",
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
