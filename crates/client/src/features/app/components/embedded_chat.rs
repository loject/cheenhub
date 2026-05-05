//! Embedded chat panel for mixed voice rooms.

use dioxus::prelude::*;

/// Renders the static lower chat panel.
#[component]
pub(crate) fn EmbeddedChat() -> Element {
    rsx! {
        div { id: "embedded-chat", class: "embedded-chat h-0 shrink-0 translate-y-6 overflow-hidden border-t border-transparent bg-[rgba(9,9,11,.86)] opacity-0 shadow-[0_-1px_0_rgba(255,255,255,0.025),0_-24px_70px_rgba(0,0,0,0.22)] backdrop-blur-[18px] transition-[height,opacity,transform,border-color] duration-[340ms] ease-[cubic-bezier(0.22,1,0.36,1)]",
            div { class: "flex h-full min-h-0 flex-col",
                div { class: "chat-resize-handle flex h-3.5 shrink-0 cursor-ns-resize touch-none items-center justify-center", role: "separator", "aria-orientation": "horizontal", "aria-label": "Изменить высоту чата", title: "Потяните, чтобы изменить высоту чата" }
                div { class: "min-h-0 flex-1 overflow-y-auto p-4 pt-2",
                    div { class: "space-y-4",
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "Ч" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "chingiz" }
                                    span { class: "text-[10px] text-zinc-600", "12:42" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Сейчас тестируем поведение голосовой комнаты вместе с текстовым каналом." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "В" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "valeria" }
                                    span { class: "text-[10px] text-zinc-600", "12:43" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Сообщения остаются внизу и не перекрывают участников." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "A" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "alex" }
                                    span { class: "text-[10px] text-zinc-600", "12:45" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Для компактного режима можно оставить только последние сообщения." }
                            }
                        }
                    }
                }
                div { class: "shrink-0 border-t border-zinc-800/80 bg-zinc-950/35 p-3",
                    div { class: "chat-input-wrap flex items-end gap-2 rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.8)] p-2 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                        textarea { rows: "1", placeholder: "Сообщение в & Лобби", class: "max-h-28 min-h-10 flex-1 resize-none bg-transparent px-2 py-2 text-[13px] text-zinc-100 outline-none placeholder:text-zinc-600" }
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
}
