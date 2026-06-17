//! Компонент поверхности чата комнаты.

use dioxus::prelude::*;

use crate::features::app::components::app_shell::ActiveRoom;

use super::panel::ChatRoomPanel;

/// Визуальный режим поверхности чата комнаты.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoomChatSurfaceMode {
    /// Full room workspace chat.
    Full,
    /// Embedded lower panel for mixed rooms.
    Embedded,
}

/// Рендерит чат в полном или встроенном виде.
#[component]
pub(crate) fn RoomChatSurface(
    server_id: String,
    room: ActiveRoom,
    mode: RoomChatSurfaceMode,
) -> Element {
    match mode {
        RoomChatSurfaceMode::Full => rsx! {
            div { id: "text-room-view", class: "text-room-view hidden min-h-0 flex-1 flex-col",
                ChatRoomPanel { server_id, room, compact: false }
            }
        },
        RoomChatSurfaceMode::Embedded => rsx! {
            div { id: "embedded-chat", class: "embedded-chat h-0 shrink-0 translate-y-6 overflow-hidden border-t border-transparent bg-[rgba(9,9,11,.86)] opacity-0 shadow-[0_-1px_0_rgba(255,255,255,0.025),0_-24px_70px_rgba(0,0,0,0.22)] backdrop-blur-[18px] transition-[height,opacity,transform,border-color] duration-[340ms] ease-[cubic-bezier(0.22,1,0.36,1)]",
                div { class: "flex h-full min-h-0 flex-col",
                    div { class: "chat-resize-handle flex h-3.5 shrink-0 cursor-ns-resize touch-none items-center justify-center", role: "separator", "aria-orientation": "horizontal", "aria-label": "Потяните, чтобы изменить высоту чата" }
                    ChatRoomPanel { server_id, room, compact: true }
                }
            }
        },
    }
}
