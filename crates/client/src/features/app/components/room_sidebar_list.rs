//! Отображение списка комнат и отправка локальных команд в scope сервера.

use super::room_list_item::RoomListItem;
use super::server_rooms_action_error::ServerRoomsActionError;
use super::server_rooms_load_error::ServerRoomsLoadError;
use super::server_rooms_loading::ServerRoomsLoading;
use super::server_rooms_state::ServerWorkspace;
use crate::features::voice_chat::VoiceConnectionHandle;
use cheenhub_contracts::rest::ServerRoomSummary;
use dioxus::prelude::*;

/// Намерения пользователя в списке комнат одного сервера.
#[derive(Clone)]
pub(super) enum RoomSidebarAction {
    /// Открыть рабочую область выбранной комнаты.
    Select(ServerRoomSummary),
    /// Открыть форму создания комнаты.
    Create,
    /// Открыть меню комнаты либо свободного места списка.
    OpenMenu {
        /// Идентификатор комнаты; `None` обозначает пустое место.
        room_id: Option<String>,
        /// Горизонтальная координата относительно окна в CSS pixels.
        x: f64,
        /// Вертикальная координата относительно окна в CSS pixels.
        y: f64,
    },
}

/// Рендерит загрузку, ошибку, пустой список или комнаты с доступными действиями.
#[component]
pub(super) fn RoomSidebarList(
    server_id: String,
    rooms: Vec<ServerRoomSummary>,
    active_workspace: Option<ServerWorkspace>,
    can_manage_rooms: bool,
    deleting_room_id: Option<String>,
    settings_workspace_active: bool,
    section_title_class: &'static str,
    is_loading_rooms: bool,
    load_error: Option<String>,
    is_retrying_rooms: bool,
    action_error: String,
    on_retry: EventHandler<()>,
    on_action: EventHandler<RoomSidebarAction>,
) -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    rsx! {
                div {
                    class: "min-h-0 flex-1 overflow-y-auto p-3",
                    oncontextmenu: move |event| {
                        if !can_manage_rooms || is_loading_rooms || load_error.is_some() { return; }
                        event.prevent_default();
                        let point = event.client_coordinates();
                        on_action.call(RoomSidebarAction::OpenMenu { room_id: None, x: point.x, y: point.y });
                    },
                    div { class: "mb-1.5 flex items-center justify-between px-1 text-[10px] font-medium uppercase tracking-[0.22em] text-zinc-600",
                        span { class: section_title_class, "Комнаты" }
                        if can_manage_rooms && !is_loading_rooms && load_error.is_none() {
                            button {
                                r#type: "button",
                                class: "rounded-md p-1 text-zinc-600 hover:bg-zinc-900 hover:text-zinc-300",
                                "aria-label": "Создать комнату",
                                onclick: move |_| on_action.call(RoomSidebarAction::Create),
                                svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                                }
                            }
                        }
                    }

                    if is_loading_rooms {
                        ServerRoomsLoading {}
                    } else if let Some(error) = load_error.clone() {
                        ServerRoomsLoadError {
                            message: error,
                            is_retrying: is_retrying_rooms,
                            on_retry: {
                                let retry = on_retry;
                                let retry_server_id = server_id.clone();
                                move |_| {
                                    info!(server_id = %retry_server_id, "retrying server rooms load");
                                    retry.call(());
                                }
                            },
                        }
                    } else if rooms.is_empty() {
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/70 p-3",
                            p { class: "text-[12px] font-medium text-zinc-100", "Комнат пока нет" }
                            p { class: "mt-1 text-[11px] leading-5 text-zinc-500",
                                if can_manage_rooms {
                                    "Создай первую комнату для этого сервера."
                                } else {
                                    "Владелец сервера еще не создал комнаты."
                                }
                            }
                            if can_manage_rooms {
                                button {
                                    r#type: "button",
                                    class: "mt-3 flex h-9 w-full items-center justify-center rounded-xl bg-accent px-3 text-[12px] font-semibold text-white transition hover:bg-blue-400",
                                    onclick: move |_| on_action.call(RoomSidebarAction::Create),
                                    "Создать комнату"
                                }
                            }
                        }
                    } else {
                        div { class: "space-y-1",
                            for room in rooms.iter().cloned() {
                                RoomListItem {
                                    key: "{room.id}",
                                    room: room.clone(),
                                    is_active: matches!(active_workspace, Some(ServerWorkspace::Room(ref id)) if id == &room.id),
                                    can_manage_rooms,
                                    is_deleting: deleting_room_id.clone().as_deref() == Some(room.id.as_str()),
                                    voice_participants: voice.room_participants(&server_id, &room.id).unwrap_or_default(),
                                    compact_when_settings_active: settings_workspace_active,
                                    on_select: {
                                        let selected = room.clone();
                                        move |_| on_action.call(RoomSidebarAction::Select(selected.clone()))
                                    },
                                    on_menu: {
                                        let target_id = room.id.clone();
                                        move |(x, y)| on_action.call(RoomSidebarAction::OpenMenu { room_id: Some(target_id.clone()), x, y })
                                    },
                                }
                            }
                        }
                    }

                    if !action_error.is_empty() {
                        ServerRoomsActionError { message: action_error.clone() }
                    }
                }
    }
}
