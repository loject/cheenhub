//! Room chat header component.

use cheenhub_contracts::rest::ServerRoomKind;
use dioxus::prelude::*;

use crate::features::app::components::app_shell::ActiveRoom;

/// Renders the full room chat header.
#[component]
pub(crate) fn RoomChatHeader(room: ActiveRoom) -> Element {
    let marker = if room.kind == ServerRoomKind::TextAndVoice {
        "&"
    } else {
        "#"
    };
    let description = if room.kind == ServerRoomKind::TextAndVoice {
        "Текстовая лента этой комнаты доступна сразу, а голос можно подключить отдельно."
    } else {
        "Текстовая лента этой комнаты хранит обсуждения, ссылки и короткие заметки."
    };

    rsx! {
        div { class: "shrink-0 p-5 pb-0 lg:p-6 lg:pb-0",
            div { class: "mx-auto w-full max-w-3xl rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-5 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                div { class: "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950/80 px-3 py-1 text-[10px] uppercase tracking-[0.22em] text-zinc-500",
                    span { class: "h-1.5 w-1.5 rounded-full bg-zinc-600" }
                    if room.kind == ServerRoomKind::TextAndVoice {
                        "Текст и голос"
                    } else {
                        "Текстовая комната"
                    }
                }
                h2 { class: "text-2xl font-semibold text-zinc-50", "{marker} {room.name}" }
                p { class: "mt-2 text-[14px] leading-6 text-zinc-500", "{description}" }
            }
        }
    }
}
