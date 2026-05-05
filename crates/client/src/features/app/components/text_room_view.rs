//! Text room view component.

use cheenhub_contracts::rest::ServerRoomKind;
use dioxus::prelude::*;

use super::app_shell::ActiveRoom;

/// Renders the full-height text room view used by text-capable rooms.
#[component]
pub(crate) fn TextRoomView(room: ActiveRoom) -> Element {
    let marker = if room.kind == ServerRoomKind::TextAndVoice {
        "&"
    } else {
        "#"
    };
    let description = if room.kind == ServerRoomKind::TextAndVoice {
        "Текстовая лента этой комнаты доступна сразу, а голос можно подключить отдельно."
    } else {
        "Текстовая лента этой комнаты будет хранить обсуждения, ссылки и короткие заметки."
    };

    rsx! {
        div { id: "text-room-view", class: "text-room-view hidden min-h-0 flex-1 flex-col",
            div { class: "min-h-0 flex-1 overflow-y-auto p-5 lg:p-6",
                div { class: "mx-auto flex w-full max-w-3xl flex-col gap-5",
                    div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/80 p-5 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                        div { class: "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950/80 px-3 py-1 text-[10px] uppercase tracking-[0.22em] text-zinc-500",
                            span { class: "h-1.5 w-1.5 rounded-full bg-zinc-600" }
                            if room.kind == ServerRoomKind::TextAndVoice {
                                "Текст и голос"
                            } else {
                                "Текстовая комната"
                            }
                        }
                        h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "{marker} {room.name}" }
                        p { class: "mt-2 text-[14px] leading-6 text-zinc-500", "{description}" }
                    }
                    div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/60 p-6 text-center",
                        p { class: "text-[13px] font-medium text-zinc-100", "Сообщений пока нет" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                            "Здесь появятся новые сообщения комнаты."
                        }
                    }
                }
            }
            div { class: "shrink-0 border-t border-zinc-800/80 bg-zinc-950/55 p-4 backdrop-blur-xl",
                div { class: "chat-input-wrap mx-auto flex max-w-3xl items-end gap-2 rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                    textarea { id: "text-room-input", rows: "1", placeholder: "Сообщение в {marker} {room.name}", class: "max-h-28 min-h-10 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600" }
                    button { r#type: "button", class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-accent text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400", "aria-label": "Отправить сообщение",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 12 3.269 3.126A59.77 59.77 0 0 1 21.485 12 59.768 59.768 0 0 1 3.27 20.876L6 12Zm0 0h7.5" }
                        }
                    }
                }
            }
        }
    }
}
