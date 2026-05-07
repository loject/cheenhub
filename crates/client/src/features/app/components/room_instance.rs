//! Per-room authenticated app workspace.

use dioxus::prelude::*;

use super::app_shell::{ActiveRoom, ServerShellState};
use super::room_header::RoomHeader;
use super::voice_controls::VoiceControls;
use super::voice_stage::VoiceStage;
use crate::features::text_chat::{RoomChatSurface, RoomChatSurfaceMode};

/// Renders one room workspace with local UI state scoped to that room.
#[component]
pub(crate) fn RoomInstance(
    server_id: String,
    room: ActiveRoom,
    active: bool,
    mut chat_open_by_room: Signal<Vec<(String, bool)>>,
    on_state_change: EventHandler<(String, ServerShellState)>,
) -> Element {
    let wrapper_class = if active { "contents" } else { "hidden" };
    let room_id = room.id.clone();
    let chat_open = chat_open_for_room(&chat_open_by_room(), &room_id);
    let chat_open_attr = if chat_open { "true" } else { "false" };
    let chat_label = if chat_open {
        "Скрыть текстовый чат"
    } else {
        "Открыть текстовый чат"
    };

    rsx! {
        div { class: wrapper_class,
            section { class: "voice-shell relative flex min-w-0 flex-1 flex-col bg-zinc-950/35",
                RoomHeader { room: room.clone() }
                div { class: "content-split flex min-h-0 flex-1 flex-col",
                    VoiceStage {}
                    RoomChatSurface {
                        server_id: server_id.clone(),
                        room: room.clone(),
                        mode: RoomChatSurfaceMode::Embedded,
                    }
                }
                RoomChatSurface {
                    server_id: server_id.clone(),
                    room: room.clone(),
                    mode: RoomChatSurfaceMode::Full,
                }
                button {
                    r#type: "button",
                    class: "chat-corner-toggle group absolute bottom-5 left-5 z-40 flex h-11 w-11 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/85 text-zinc-300 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur-xl transition-[background-color,border-color,color,transform,box-shadow] duration-[180ms] hover:-translate-y-0.5 hover:border-accent/35 hover:bg-accent/10 hover:text-zinc-100",
                    "aria-label": chat_label,
                    "aria-expanded": chat_open_attr,
                    onclick: move |_| {
                        let next_chat_open = !chat_open;
                        let mut next_chat_open_by_room = chat_open_by_room();

                        if let Some((_, saved_chat_open)) = next_chat_open_by_room
                            .iter_mut()
                            .find(|(saved_room_id, _)| saved_room_id == &room_id)
                        {
                            *saved_chat_open = next_chat_open;
                        } else {
                            next_chat_open_by_room.push((room_id.clone(), next_chat_open));
                        }

                        chat_open_by_room.set(next_chat_open_by_room);
                        on_state_change.call((
                            server_id.clone(),
                            ServerShellState {
                                chat_open: next_chat_open,
                                room_kind: super::app_shell::room_kind_attr(room.kind),
                            },
                        ));
                    },
                    span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-0 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100",
                        span { class: "chat-tooltip-open", "Открыть текстовый чат" }
                        span { class: "chat-tooltip-close", "Скрыть текстовый чат" }
                    }
                    svg { class: "h-4 w-4 transition-transform duration-200", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m18 15-6-6-6 6" }
                    }
                }
                VoiceControls {}
            }
        }
    }
}

fn chat_open_for_room(chat_open_by_room: &[(String, bool)], room_id: &str) -> bool {
    chat_open_by_room
        .iter()
        .find_map(|(saved_room_id, chat_open)| (saved_room_id == room_id).then_some(*chat_open))
        .unwrap_or(false)
}
