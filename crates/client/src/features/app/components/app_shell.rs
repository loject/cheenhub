//! Main authenticated CheenHub application shell.

use dioxus::prelude::*;

use super::embedded_chat::EmbeddedChat;
use super::room_header::RoomHeader;
use super::room_sidebar::RoomSidebar;
use super::server_rail::ServerRail;
use super::text_room_view::TextRoomView;
use super::voice_controls::VoiceControls;
use super::voice_stage::VoiceStage;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ActiveRoom {
    pub(crate) kind: &'static str,
    pub(crate) name: &'static str,
}

/// Renders the static room UI shell.
#[component]
pub(crate) fn AppShell() -> Element {
    let mut is_chat_open = use_signal(|| false);
    let mut active_room = use_signal(|| ActiveRoom {
        kind: "mixed",
        name: "Лобби",
    });
    let chat_open = if is_chat_open() { "true" } else { "false" };
    let chat_label = if is_chat_open() {
        "Скрыть текстовый чат"
    } else {
        "Открыть текстовый чат"
    };

    rsx! {
        main {
            id: "app-shell",
            "data-chat-open": chat_open,
            "data-room-kind": active_room().kind,
            class: "grid-bg flex h-screen min-h-0 w-full overflow-hidden bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            ServerRail {}
            RoomSidebar {
                active_room: active_room().name,
                on_select_room: move |room: ActiveRoom| {
                    active_room.set(room);
                    is_chat_open.set(false);
                },
            }
            section { class: "voice-shell relative flex min-w-0 flex-1 flex-col bg-zinc-950/35",
                RoomHeader { room: active_room() }
                div { class: "content-split flex min-h-0 flex-1 flex-col",
                    VoiceStage {}
                    EmbeddedChat {}
                }
                TextRoomView { room: active_room() }
                button {
                    r#type: "button",
                    class: "chat-corner-toggle group absolute bottom-5 left-5 z-40 flex h-11 w-11 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/85 text-zinc-300 shadow-[0_18px_50px_rgba(0,0,0,0.38)] backdrop-blur-xl transition-[background-color,border-color,color,transform,box-shadow] duration-[180ms] hover:-translate-y-0.5 hover:border-accent/35 hover:bg-accent/10 hover:text-zinc-100",
                    "aria-label": chat_label,
                    "aria-expanded": chat_open,
                    onclick: move |_| is_chat_open.set(!is_chat_open()),
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
