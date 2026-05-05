//! Per-server authenticated app workspace.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use super::app_shell::{ActiveRoom, AppModal, ServerShellState};
use super::room_instance::RoomInstance;
use super::room_sidebar::RoomSidebar;

const ROOMS: [ActiveRoom; 6] = [
    ActiveRoom {
        kind: "mixed",
        name: "Лобби",
    },
    ActiveRoom {
        kind: "mixed-chat",
        name: "Разработка",
    },
    ActiveRoom {
        kind: "mixed-chat",
        name: "Тест звука",
    },
    ActiveRoom {
        kind: "text",
        name: "общий",
    },
    ActiveRoom {
        kind: "text",
        name: "релизы",
    },
    ActiveRoom {
        kind: "text",
        name: "баги",
    },
];

/// Renders one server workspace with local UI state scoped to that server.
#[component]
pub(crate) fn ServerInstance(
    server: ServerSummary,
    active: bool,
    on_state_change: EventHandler<(String, ServerShellState)>,
    on_open_modal: EventHandler<AppModal>,
) -> Element {
    let mut active_room = use_signal(|| ActiveRoom {
        kind: "mixed",
        name: "Лобби",
    });
    let chat_open_by_room = use_signal(|| vec![false; ROOMS.len()]);
    let server_id = server.id.clone();
    let room_select_server_id = server_id.clone();
    let wrapper_class = if active { "contents" } else { "hidden" };

    rsx! {
        div { class: wrapper_class,
            RoomSidebar {
                server: server.clone(),
                active_room: active_room().name,
                on_create_invite: move |(server_id, server_name): (String, String)| {
                    on_open_modal.call(AppModal::InviteLink {
                        server_id,
                        server_name,
                    });
                },
                on_select_room: move |room: ActiveRoom| {
                    active_room.set(room);

                    if active {
                        on_state_change.call((
                            room_select_server_id.clone(),
                            ServerShellState {
                                chat_open: chat_open_for_room(&chat_open_by_room(), room),
                                room_kind: room.kind,
                            },
                        ));
                    }
                },
            }
            for (room_index, room) in ROOMS.into_iter().enumerate() {
                RoomInstance {
                    key: "{server.id}:{room.kind}:{room.name}",
                    server_id: server_id.clone(),
                    room,
                    room_index,
                    active: active && active_room() == room,
                    chat_open_by_room,
                    on_state_change,
                }
            }
        }
    }
}

fn chat_open_for_room(chat_open_by_room: &[bool], room: ActiveRoom) -> bool {
    chat_open_by_room
        .get(room_index(room))
        .copied()
        .unwrap_or(false)
}

fn room_index(room: ActiveRoom) -> usize {
    ROOMS
        .iter()
        .position(|candidate| *candidate == room)
        .unwrap_or(0)
}
