//! Компонент поверхности чата комнаты.

use std::rc::Rc;

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
    embedded_resizing: bool,
    on_embedded_resize_start: EventHandler<(f64, Option<f64>)>,
) -> Element {
    match mode {
        RoomChatSurfaceMode::Full => rsx! {
            div { id: "text-room-view", class: "text-room-view hidden min-h-0 flex-1 flex-col",
                ChatRoomPanel { server_id, room, compact: false }
            }
        },
        RoomChatSurfaceMode::Embedded => {
            let mut embedded_element = use_signal(|| None::<Rc<MountedData>>);
            let resizing_attr = if embedded_resizing { "true" } else { "false" };

            rsx! {
                div {
                    id: "embedded-chat",
                    class: "embedded-chat h-0 shrink-0 translate-y-6 overflow-hidden border-t border-transparent bg-[rgba(9,9,11,.86)] opacity-0 shadow-[0_-1px_0_rgba(255,255,255,0.025),0_-24px_70px_rgba(0,0,0,0.22)] backdrop-blur-[18px] transition-[height,opacity,transform,border-color] duration-[340ms] ease-[cubic-bezier(0.22,1,0.36,1)]",
                    "data-resizing": resizing_attr,
                    onmounted: move |event| embedded_element.set(Some(event.data.clone())),
                div { class: "flex h-full min-h-0 flex-col",
                    div {
                        class: "chat-resize-handle flex h-3.5 shrink-0 cursor-ns-resize touch-none items-center justify-center",
                        role: "separator",
                        "aria-orientation": "horizontal",
                        "aria-label": "Потяните, чтобы изменить высоту чата",
                        onpointerdown: move |event| {
                            event.prevent_default();
                            event.stop_propagation();
                            let point = event.client_coordinates();
                            let element = embedded_element.cloned();

                            spawn(async move {
                                let measured_height_px = match element {
                                    Some(element) => element
                                        .get_client_rect()
                                        .await
                                        .ok()
                                        .map(|rect| rect.size.height),
                                    None => None,
                                };
                                on_embedded_resize_start.call((point.y, measured_height_px));
                            });
                        },
                    }
                    ChatRoomPanel { server_id, room, compact: true }
                }
            }
            }
        }
    }
}
