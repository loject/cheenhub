//! Static text room view component.

use dioxus::prelude::*;

use super::app_shell::ActiveRoom;

/// Renders the full-height text room view used by text-only rooms.
#[component]
pub(crate) fn TextRoomView(room: ActiveRoom) -> Element {
    let marker = if room.kind == "mixed-chat" { "&" } else { "#" };
    let description = match room.name {
        "релизы" => "Канал для анонсов версий, changelog и ссылок на сборки.",
        "баги" => "Канал для багрепортов, заметок по поломкам и проверки фиксов.",
        "Разработка" => {
            "Текстовый канал комнаты с возможностью подключиться к голосу, когда это нужно."
        }
        "Тест звука" => {
            "Комната для проверки микрофона, наушников и коротких сообщений по настройке звука."
        }
        _ => "Общий канал для обсуждений, ссылок и коротких сообщений без голосовой сцены.",
    };

    rsx! {
        div { id: "text-room-view", class: "text-room-view hidden min-h-0 flex-1 flex-col",
            div { class: "min-h-0 flex-1 overflow-y-auto p-5 lg:p-6",
                div { class: "mx-auto flex w-full max-w-3xl flex-col gap-5",
                    div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/80 p-5 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                        div { class: "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950/80 px-3 py-1 text-[10px] uppercase tracking-[0.22em] text-zinc-500",
                            span { class: "h-1.5 w-1.5 rounded-full bg-zinc-600" }
                            "Текстовая комната"
                        }
                        h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "{marker} {room.name}" }
                        p { class: "mt-2 text-[14px] leading-6 text-zinc-500", "{description}" }
                    }
                    div { class: "text-room-panel hidden space-y-4 data-[active=true]:block", "data-text-room-panel": "общий", "data-active": if room.name == "общий" { "true" } else { "false" },
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "Ч" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "chingiz" }
                                    span { class: "text-[10px] text-zinc-600", "13:02" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Это полноценный интерфейс только текстовой комнаты: без плиток участников и без голосовых controls." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "В" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "valeria" }
                                    span { class: "text-[10px] text-zinc-600", "13:04" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "При клике по # комнате правая область переключается целиком, а не открывает нижний чат." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "A" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "alex" }
                                    span { class: "text-[10px] text-zinc-600", "13:07" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Список сообщений может занимать всю высоту, поэтому читать длинные треды удобнее." }
                            }
                        }
                    }
                    div { class: "text-room-panel hidden space-y-4 data-[active=true]:block", "data-text-room-panel": "релизы", "data-active": if room.name == "релизы" { "true" } else { "false" },
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "Ч" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "chingiz" }
                                    span { class: "text-[10px] text-zinc-600", "10:12" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "v0.9.2: добавлены compact-комнаты, нижний чат и resize-поведение." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "M" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "mira" }
                                    span { class: "text-[10px] text-zinc-600", "10:16" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Нужен changelog с короткими пунктами и ссылкой на GitHub Release." }
                            }
                        }
                    }
                    div { class: "text-room-panel hidden space-y-4 data-[active=true]:block", "data-text-room-panel": "баги", "data-active": if room.name == "баги" { "true" } else { "false" },
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "D" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "den" }
                                    span { class: "text-[10px] text-zinc-600", "09:28" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Если открыть контекстное меню у плитки рядом с краем, оно больше не должно накладываться на кнопку троеточия." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "A" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "alex" }
                                    span { class: "text-[10px] text-zinc-600", "09:31" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Проверить ещё мобильную ширину и скролл в popover участников." }
                            }
                        }
                    }
                    div { class: "text-room-panel hidden space-y-4 data-[active=true]:block", "data-text-room-panel": "Разработка", "data-active": if room.name == "Разработка" { "true" } else { "false" },
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "Ч" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "chingiz" }
                                    span { class: "text-[10px] text-zinc-600", "14:10" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Это &-комната: текст доступен сразу, а к голосу можно подключиться отдельной кнопкой сверху." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "M" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "mira" }
                                    span { class: "text-[10px] text-zinc-600", "14:13" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Удобно для рабочих комнат: сначала читаешь контекст, потом входишь в голос." }
                            }
                        }
                    }
                    div { class: "text-room-panel hidden space-y-4 data-[active=true]:block", "data-text-room-panel": "Тест звука", "data-active": if room.name == "Тест звука" { "true" } else { "false" },
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "A" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "alex" }
                                    span { class: "text-[10px] text-zinc-600", "11:24" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Проверяю микрофон. Если всё нормально, можно нажать трубку сверху и зайти в голос." }
                            }
                        }
                        div { class: "flex gap-3",
                            div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-800 text-[12px] font-bold text-zinc-100", "D" }
                            div { class: "min-w-0 flex-1",
                                div { class: "mb-1 flex items-center gap-2",
                                    span { class: "text-[12px] font-semibold text-zinc-100", "den" }
                                    span { class: "text-[10px] text-zinc-600", "11:25" }
                                }
                                div { class: "message-bubble rounded-[20px] border border-zinc-800 bg-[rgba(39,39,42,.72)] px-3 py-2 text-[13px] leading-5 text-zinc-300 transition-[border-color,background] duration-200 hover:border-white/15 hover:bg-[rgba(39,39,42,.84)]", "Слышно тихо, попробуй поднять gain на пару делений." }
                            }
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
