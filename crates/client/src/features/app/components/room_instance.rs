//! Per-room authenticated app workspace.

use dioxus::prelude::*;

use super::app_shell::{ActiveRoom, ServerShellState};
use super::embedded_chat::EmbeddedChat;
use super::room_header::RoomHeader;
use super::text_room_view::TextRoomView;
use super::voice_controls::VoiceControls;
use super::voice_stage::VoiceStage;

/// Renders one room workspace with local UI state scoped to that room.
#[component]
pub(crate) fn RoomInstance(
    server_id: String,
    room: ActiveRoom,
    room_index: usize,
    active: bool,
    mut chat_open_by_room: Signal<Vec<bool>>,
    on_state_change: EventHandler<(String, ServerShellState)>,
) -> Element {
    let wrapper_class = if active { "contents" } else { "hidden" };
    let chat_open = chat_open_by_room()
        .get(room_index)
        .copied()
        .unwrap_or(false);
    let chat_open_attr = if chat_open { "true" } else { "false" };
    let chat_label = if chat_open {
        "Скрыть текстовый чат"
    } else {
        "Открыть текстовый чат"
    };

    rsx! {
        div { class: wrapper_class,
            section { class: "voice-shell relative flex min-w-0 flex-1 flex-col bg-zinc-950/35",
                RoomHeader { room }
                div { class: "content-split flex min-h-0 flex-1 flex-col",
                    VoiceStage {}
                    EmbeddedChat {}
                }
                TextRoomView { room }
                button {
                    r#type: "button",
                    class: "chat-corner-toggle group absolute bottom-5 left-5 z-40 flex h-11 w-11 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/85 text-zinc-300 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur-xl transition-[background-color,border-color,color,transform,box-shadow] duration-[180ms] hover:-translate-y-0.5 hover:border-accent/35 hover:bg-accent/10 hover:text-zinc-100",
                    "aria-label": chat_label,
                    "aria-expanded": chat_open_attr,
                    onclick: move |_| {
                        let next_chat_open = !chat_open;
                        let mut next_chat_open_by_room = chat_open_by_room();

                        if let Some(saved_chat_open) = next_chat_open_by_room.get_mut(room_index) {
                            *saved_chat_open = next_chat_open;
                        }

                        chat_open_by_room.set(next_chat_open_by_room);
                        on_state_change.call((
                            server_id.clone(),
                            ServerShellState {
                                chat_open: next_chat_open,
                                room_kind: room.kind,
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
