//! Рабочая область аутентифицированного приложения для одной комнаты.

use std::rc::Rc;

use cheenhub_contracts::rest::ServerRoomKind;
use dioxus::prelude::*;

use super::app_shell::{ActiveRoom, ServerShellState};
use super::room_header::RoomHeader;
use super::workspace_split::{
    EMBEDDED_CHAT_DEFAULT_WORKSPACE_RATIO, clamp_embedded_chat_height, finish_embedded_chat_resize,
};
use crate::features::text_chat::{RoomChatSurface, RoomChatSurfaceMode};
use crate::features::voice_chat::{VoiceConnectionHandle, VoiceRoomSurface};

/// Рендерит одну рабочую область комнаты с локальным UI-состоянием, ограниченным этой комнатой.
#[component]
pub(crate) fn RoomInstance(
    server_id: String,
    room: ActiveRoom,
    active: bool,
    mobile_workspace_open: bool,
    mut chat_open_by_room: Signal<Vec<(String, bool)>>,
    on_state_change: EventHandler<(String, ServerShellState)>,
    on_mobile_back: EventHandler<()>,
) -> Element {
    let mut embedded_chat_height_px = use_signal(|| None::<f64>);
    let mut embedded_chat_resize_origin = use_signal(|| None::<(f64, f64, f64)>);
    let mut content_split_element = use_signal(|| None::<Rc<MountedData>>);
    let wrapper_class = if active {
        "room-workspace-shell contents"
    } else {
        "room-workspace-shell hidden"
    };
    let mobile_workspace_open_attr = if mobile_workspace_open {
        "true"
    } else {
        "false"
    };
    let room_id = room.id.clone();
    let chat_open = chat_open_for_room(&chat_open_by_room(), &room_id);
    let chat_open_attr = if chat_open { "true" } else { "false" };
    let voice = use_context::<VoiceConnectionHandle>();
    let voice_state = voice.state();
    let voice_room_active = voice_state.is_active_room(&server_id, &room.id);
    let voice_room_connected = voice_state.is_connected_room(&server_id, &room.id);
    let chat_resizing = embedded_chat_resize_origin().is_some();
    let chat_resizing_attr = if chat_resizing { "true" } else { "false" };
    let chat_label = if chat_open {
        "Скрыть текстовый чат"
    } else {
        "Открыть текстовый чат"
    };
    let workspace_style = embedded_chat_height_px()
        .map(|height_px| format!("--embedded-chat-height: {}px;", height_px.round()))
        .unwrap_or_default();
    let full_chat_active = active
        && (matches!(room.kind, ServerRoomKind::Text)
            || matches!(room.kind, ServerRoomKind::TextAndVoice) && !voice_room_active);
    let embedded_chat_active = active && voice_room_active && chat_open;

    rsx! {
        div { class: wrapper_class, "data-mobile-workspace-open": mobile_workspace_open_attr,
            section {
                class: "room-workspace voice-shell relative flex min-w-0 flex-1 flex-col bg-zinc-950/35",
                style: "{workspace_style}",
                "data-room-kind": super::app_shell::room_kind_attr(room.kind),
                "data-voice-room-active": if voice_room_active { "true" } else { "false" },
                "data-voice-connected": if voice_room_connected { "true" } else { "false" },
                RoomHeader {
                    server_id: server_id.clone(),
                    room: room.clone(),
                    on_mobile_back,
                }
                div {
                    class: "content-split flex min-h-0 flex-1 flex-col",
                    "data-chat-resizing": chat_resizing_attr,
                    onmounted: move |event| content_split_element.set(Some(event.data.clone())),
                    onpointermove: move |event| {
                        let Some((start_y, start_height, workspace_height)) = embedded_chat_resize_origin() else {
                            return;
                        };

                        event.prevent_default();
                        let point = event.client_coordinates();
                        let next_height = clamp_embedded_chat_height(
                            start_height + start_y - point.y,
                            workspace_height,
                        );
                        embedded_chat_height_px.set(Some(next_height));
                    },
                    onpointerup: {
                        let resize_room_id = room.id.clone();
                        move |_| {
                            finish_embedded_chat_resize(
                                embedded_chat_resize_origin,
                                embedded_chat_height_px,
                                &resize_room_id,
                            );
                        }
                    },
                    onpointerleave: {
                        let resize_room_id = room.id.clone();
                        move |_| {
                            finish_embedded_chat_resize(
                                embedded_chat_resize_origin,
                                embedded_chat_height_px,
                                &resize_room_id,
                            );
                        }
                    },
                    VoiceRoomSurface {
                        server_id: server_id.clone(),
                        room: room.clone(),
                    }
                    RoomChatSurface {
                        server_id: server_id.clone(),
                        room: room.clone(),
                        mode: RoomChatSurfaceMode::Embedded,
                        active: embedded_chat_active,
                        embedded_resizing: chat_resizing,
                        on_embedded_resize_start: move |(start_y, measured_height_px): (f64, Option<f64>)| {
                            let split_element = content_split_element.cloned();
                            spawn(async move {
                                let workspace_height = match split_element {
                                    Some(element) => element
                                        .get_client_rect()
                                        .await
                                        .ok()
                                        .map(|rect| rect.size.height),
                                    None => None,
                                }
                                .filter(|height| *height > 0.0)
                                .unwrap_or_else(|| {
                                    measured_height_px
                                        .filter(|height| *height > 0.0)
                                        .unwrap_or(1.0)
                                        / EMBEDDED_CHAT_DEFAULT_WORKSPACE_RATIO
                                });
                                let start_height = measured_height_px
                                    .filter(|height_px| *height_px > 0.0)
                                    .or(embedded_chat_height_px())
                                    .unwrap_or(workspace_height * EMBEDDED_CHAT_DEFAULT_WORKSPACE_RATIO);

                                embedded_chat_resize_origin.set(Some((
                                    start_y,
                                    clamp_embedded_chat_height(start_height, workspace_height),
                                    workspace_height,
                                )));
                            });
                        },
                    }
                }
                RoomChatSurface {
                    server_id: server_id.clone(),
                    room: room.clone(),
                    mode: RoomChatSurfaceMode::Full,
                    active: full_chat_active,
                    embedded_resizing: false,
                    on_embedded_resize_start: move |(_, _)| {},
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
