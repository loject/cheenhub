//! Per-server room state and workspace coordination.

use cheenhub_contracts::rest::{ServerRoomKind, ServerRoomSummary, ServerSummary};
use dioxus::prelude::*;

use crate::features::app::api;

use super::app_shell::{ActiveRoom, AppModal, ServerShellState, room_kind_attr};
use super::room_editor_modal::RoomEditorModal;
use super::room_instance::RoomInstance;
use super::server_context_menu::ServerContextMenu;

#[derive(Clone, PartialEq)]
enum RoomModal {
    Create,
    Edit(ServerRoomSummary),
}

/// Owns room state for one server and renders the room sidebar and active room.
#[component]
pub(crate) fn ServerRoomsScope(
    server: ServerSummary,
    active: bool,
    on_state_change: EventHandler<(String, ServerShellState)>,
    on_open_modal: EventHandler<AppModal>,
    on_left_server: EventHandler<String>,
) -> Element {
    let mut rooms = use_signal(Vec::<ServerRoomSummary>::new);
    let mut active_room_id = use_signal(|| None::<String>);
    let mut loaded_rooms = use_signal(|| false);
    let mut is_loading_rooms = use_signal(|| false);
    let mut room_status = use_signal(String::new);
    let mut room_action_status = use_signal(String::new);
    let mut room_modal = use_signal(|| None::<RoomModal>);
    let mut is_server_menu_open = use_signal(|| false);
    let chat_open_by_room = use_signal(Vec::<(String, bool)>::new);
    let server_id = server.id.clone();
    let delete_server_id = server.id.clone();
    let select_server_id = server.id.clone();
    let save_server_id = server.id.clone();
    let invite_server_id = server.id.clone();
    let modal_server_id = server.id.clone();
    let server_name = server.name.clone();
    let invite_server_name = server_name.clone();
    let is_owner = server.is_owner;
    let selected_room = active_room(&rooms(), active_room_id().as_deref());
    let active_room_name = selected_room
        .as_ref()
        .map(|room| room.name.clone())
        .unwrap_or_default();
    let active_voice_label = if active_room_name.is_empty() {
        "Нет активной комнаты".to_owned()
    } else {
        active_room_name
    };

    use_effect(move || {
        if loaded_rooms() {
            return;
        }
        loaded_rooms.set(true);
        is_loading_rooms.set(true);
        let request_server_id = server_id.clone();
        let state_server_id = server_id.clone();

        spawn(async move {
            match api::list_server_rooms(request_server_id).await {
                Ok(next_rooms) => {
                    let next_active_room_id = next_rooms.first().map(|room| room.id.clone());
                    active_room_id.set(next_active_room_id.clone());
                    room_status.set(String::new());
                    rooms.set(next_rooms);

                    if let Some(room_id) = next_active_room_id
                        && let Some(room) = active_room(&rooms(), Some(room_id.as_str()))
                    {
                        on_state_change.call((
                            state_server_id.clone(),
                            ServerShellState {
                                chat_open: chat_open_for_room(&chat_open_by_room(), &room.id),
                                room_kind: room_kind_attr(room.kind),
                            },
                        ));
                    }
                }
                Err(error) => room_status.set(error),
            }
            is_loading_rooms.set(false);
        });
    });

    rsx! {
        aside {
            class: "flex w-[284px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl",
            onclick: move |_| is_server_menu_open.set(false),
            div { class: "relative border-b border-zinc-800/80 p-4",
                button {
                    r#type: "button",
                    class: "flex w-full items-center justify-between rounded-2xl border border-zinc-800 bg-zinc-900/80 px-4 py-3 text-left transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-700 hover:bg-zinc-800",
                    "aria-haspopup": "menu",
                    "aria-expanded": if is_server_menu_open() { "true" } else { "false" },
                    onclick: move |event| {
                        event.stop_propagation();
                        is_server_menu_open.set(!is_server_menu_open());
                    },
                    span {
                        span { class: "block text-[13px] font-semibold tracking-[-0.02em] text-zinc-100", "{server_name}" }
                        span { class: "mt-0.5 block text-[11px] text-zinc-500",
                            if is_owner { "Владелец сервера" } else { "Участник сервера" }
                        }
                    }
                    svg { class: "h-4 w-4 text-zinc-500", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
                    }
                }
                if is_server_menu_open() {
                    ServerContextMenu {
                        server_id: server.id.clone(),
                        is_owner,
                        on_create_invite: move |_| {
                            is_server_menu_open.set(false);
                            on_open_modal.call(AppModal::InviteLink {
                                server_id: invite_server_id.clone(),
                                server_name: invite_server_name.clone(),
                            });
                        },
                        on_left_server: move |server_id: String| {
                            is_server_menu_open.set(false);
                            on_left_server.call(server_id);
                        },
                    }
                }
            }

            div { class: "min-h-0 flex-1 overflow-y-auto p-3",
                div { class: "mb-1.5 flex items-center justify-between px-1 text-[10px] font-medium uppercase tracking-[0.22em] text-zinc-600",
                    span { "Комнаты" }
                    if is_owner {
                        button {
                            r#type: "button",
                            class: "rounded-md p-1 text-zinc-600 hover:bg-zinc-900 hover:text-zinc-300",
                            "aria-label": "Создать комнату",
                            onclick: move |_| room_modal.set(Some(RoomModal::Create)),
                            svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14m-7-7h14" }
                            }
                        }
                    }
                }

                if is_loading_rooms() {
                    div { class: "space-y-2 px-1 py-2",
                        div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/80" }
                        div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/60" }
                        div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/40" }
                    }
                } else if !room_status().is_empty() {
                    div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{room_status()}"
                    }
                } else if rooms().is_empty() {
                    div { class: "rounded-xl border border-zinc-800 bg-zinc-900/70 p-3",
                        p { class: "text-[12px] font-medium text-zinc-100", "Комнат пока нет" }
                        p { class: "mt-1 text-[11px] leading-5 text-zinc-500",
                            if is_owner {
                                "Создай первую комнату для этого сервера."
                            } else {
                                "Владелец сервера еще не создал комнаты."
                            }
                        }
                        if is_owner {
                            button {
                                r#type: "button",
                                class: "mt-3 flex h-9 w-full items-center justify-center rounded-xl bg-accent px-3 text-[12px] font-semibold text-white transition hover:bg-blue-400",
                                onclick: move |_| room_modal.set(Some(RoomModal::Create)),
                                "Создать комнату"
                            }
                        }
                    }
                } else {
                    div { class: "space-y-1",
                        for room in rooms() {
                            div {
                                key: "{room.id}",
                                "data-active": if active_room_id().as_deref() == Some(room.id.as_str()) { "true" } else { "false" },
                                class: "group relative flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 data-[active=true]:border-accent/25 data-[active=true]:bg-accent/10 data-[active=true]:text-zinc-100",
                                button {
                                    r#type: "button",
                                    class: "flex min-w-0 flex-1 items-center gap-2 text-left",
                                    "aria-label": "Открыть комнату {room.name}",
                                    onclick: {
                                        let room = room.clone();
                                        let select_server_id = select_server_id.clone();
                                        move |_| {
                                            active_room_id.set(Some(room.id.clone()));

                                            if active {
                                                on_state_change.call((
                                                    select_server_id.clone(),
                                                    ServerShellState {
                                                        chat_open: chat_open_for_room(
                                                            &chat_open_by_room(),
                                                            &room.id,
                                                        ),
                                                        room_kind: room_kind_attr(room.kind),
                                                    },
                                                ));
                                            }
                                        }
                                    },
                                    span { class: room_icon_class(room.kind), "{room_icon(room.kind)}" }
                                    span { class: "truncate text-[12px] font-medium", "{room.name}" }
                                }
                                if is_owner {
                                    span { class: "ml-2 flex shrink-0 items-center gap-1 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100",
                                        button {
                                            r#type: "button",
                                            class: "rounded-md p-1 text-zinc-600 hover:bg-zinc-800 hover:text-zinc-200",
                                            "aria-label": "Изменить комнату {room.name}",
                                            onclick: {
                                                let room = room.clone();
                                                move |_| room_modal.set(Some(RoomModal::Edit(room.clone())))
                                            },
                                            svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125" }
                                            }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "rounded-md p-1 text-zinc-600 hover:bg-red-500/10 hover:text-red-200",
                                            "aria-label": "Удалить комнату {room.name}",
                                            onclick: {
                                                let room_id = room.id.clone();
                                                let delete_server_id = delete_server_id.clone();
                                                move |_| {
                                                    room_action_status.set(String::new());
                                                    let request_server_id = delete_server_id.clone();
                                                    let state_server_id = delete_server_id.clone();
                                                    let room_id = room_id.clone();

                                                    spawn(async move {
                                                        match api::delete_server_room(
                                                            request_server_id,
                                                            room_id.clone(),
                                                        )
                                                        .await
                                                        {
                                                            Ok(()) => {
                                                                let mut next_rooms = rooms();
                                                                next_rooms
                                                                    .retain(|room| room.id != room_id);
                                                                let next_active_room_id =
                                                                    if active_room_id().as_deref()
                                                                        == Some(room_id.as_str())
                                                                    {
                                                                        next_rooms
                                                                            .first()
                                                                            .map(|room| room.id.clone())
                                                                    } else {
                                                                        active_room_id()
                                                                    };
                                                                active_room_id
                                                                    .set(next_active_room_id.clone());
                                                                rooms.set(next_rooms.clone());

                                                                if let Some(next_room) = active_room(
                                                                    &next_rooms,
                                                                    next_active_room_id.as_deref(),
                                                                ) {
                                                                    on_state_change.call((
                                                                        state_server_id.clone(),
                                                                        ServerShellState {
                                                                            chat_open: chat_open_for_room(
                                                                                &chat_open_by_room(),
                                                                                &next_room.id,
                                                                            ),
                                                                            room_kind: room_kind_attr(
                                                                                next_room.kind,
                                                                            ),
                                                                        },
                                                                    ));
                                                                }
                                                            }
                                                            Err(error) => {
                                                                room_action_status.set(error);
                                                            }
                                                        }
                                                    });
                                                }
                                            },
                                            svg { class: "h-3.5 w-3.5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673A2.25 2.25 0 0 1 15.916 21H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !room_action_status().is_empty() {
                    p { class: "mt-3 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[11px] leading-4 text-red-200",
                        "{room_action_status()}"
                    }
                }
            }

            div { class: "border-t border-zinc-800/80 p-3",
                div { class: "relative mb-2 rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5",
                    div { class: "mb-2 flex items-start gap-2",
                        button { r#type: "button", class: "group relative flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-emerald-500/20 bg-emerald-500/10 text-emerald-300 hover:border-emerald-400/35 hover:bg-emerald-500/15", "aria-label": "Качество связи",
                            span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-0 z-[90] w-max min-w-[190px] translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 p-3 text-left opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100",
                                span { class: "block text-[12px] font-medium text-zinc-100", "Качество связи: хорошее" }
                                span { class: "mt-1 block text-[11px] text-zinc-500", "Пинг: 24 мс · Потери: 0.3%" }
                            }
                            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 18.5v-3.25" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8.5 18.5h7" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.25 13.75a4 4 0 0 1 5.5 0" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.5 11a8 8 0 0 1 11 0" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3.75 8.25a12 12 0 0 1 16.5 0" }
                                circle { cx: "12", cy: "18.5", r: "1.15", fill: "currentColor", stroke: "none" }
                            }
                        }
                        div { class: "min-w-0 flex-1",
                            div { class: "truncate text-[11px] font-medium text-zinc-100", "{server_name}" }
                            div { class: "mt-0.5 flex min-w-0 items-center gap-1.5 text-[11px] text-zinc-500",
                                svg { class: "h-3.5 w-3.5 shrink-0 text-zinc-600", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                                }
                                span { class: "truncate", "{active_voice_label}" }
                            }
                        }
                    }
                    div { class: "grid grid-cols-4 gap-2",
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Выключить микрофон",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                            }
                        }
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Отключить звук",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.114 5.636a9 9 0 0 1 0 12.728M16.463 8.288a5.25 5.25 0 0 1 0 7.424M6.75 8.25 10.5 4.5v15l-3.75-3.75H3.75A1.5 1.5 0 0 1 2.25 14.25v-4.5A1.5 1.5 0 0 1 3.75 8.25h3Z" }
                            }
                        }
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950/70 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900", "aria-label": "Настройки голосовой сессии",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.5 6h9.75M10.5 6a1.5 1.5 0 1 1-3 0m3 0a1.5 1.5 0 1 0-3 0M3.75 6H7.5m9 6h3.75m-3.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-9.75 0h9.75m-3 6h9.75m-9.75 0a1.5 1.5 0 0 1-3 0m3 0a1.5 1.5 0 0 0-3 0m-3.75 0H7.5" }
                            }
                        }
                        button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200", "aria-label": "Выйти из голосового чата",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-3 rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-accent text-[12px] font-bold text-white", "Ч" }
                    div { class: "min-w-0 flex-1",
                        div { class: "truncate text-[12px] font-medium text-zinc-100", "chingiz" }
                        div { class: "truncate text-[11px] text-zinc-500", "в приложении" }
                    }
                    button { r#type: "button", class: "rounded-lg p-2 text-zinc-500 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-zinc-800 hover:text-zinc-200", "aria-label": "Настройки",
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.075.04.149.083.22.127.324.2.72.226 1.075.091l1.217-.462a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.82c-.293.24-.438.613-.431.992a7.723 7.723 0 0 1 0 .255c-.007.379.138.752.431.992l1.003.82c.433.354.54.972.26 1.431l-1.296 2.247a1.125 1.125 0 0 1-1.37.49l-1.217-.462c-.355-.135-.751-.109-1.076.091a6.72 6.72 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.542-.56.94-1.11.94h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.063-.374-.313-.686-.645-.87a6.52 6.52 0 0 1-.22-.127c-.324-.2-.72-.226-1.075-.091l-1.217.462a1.125 1.125 0 0 1-1.37-.49l-1.296-2.247a1.125 1.125 0 0 1 .26-1.431l1.003-.82c.293-.24.438-.613.431-.992a6.932 6.932 0 0 1 0-.255c.007-.379-.138-.752-.431-.992l-1.003-.82a1.125 1.125 0 0 1-.26-1.431l1.296-2.247a1.125 1.125 0 0 1 1.37-.49l1.217.462c.355-.135.751-.109 1.076-.091.071-.044.145-.087.220-.128.331-.183.581-.495.644-.869l.213-1.281Z" }
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" }
                        }
                    }
                }
            }
        }

        if let Some(room) = selected_room {
            RoomInstance {
                key: "{server.id}:{room.id}",
                server_id: server.id.clone(),
                room,
                active,
                chat_open_by_room,
                on_state_change,
            }
        } else if !is_loading_rooms() {
            section { class: "flex min-w-0 flex-1 items-center justify-center bg-zinc-950/35 p-6",
                div { class: "max-w-sm text-center",
                    h2 { class: "text-[16px] font-semibold text-zinc-100", "Комнат пока нет" }
                    p { class: "mt-2 text-[13px] leading-6 text-zinc-500",
                        if server.is_owner {
                            "Создай первую комнату, чтобы участникам было куда перейти."
                        } else {
                            "Владелец сервера еще не создал комнаты."
                        }
                    }
                    if is_owner {
                        button {
                            r#type: "button",
                            class: "mt-4 inline-flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition hover:bg-blue-400",
                            onclick: move |_| room_modal.set(Some(RoomModal::Create)),
                            "Создать комнату"
                        }
                    }
                }
            }
        }
        if let Some(modal) = room_modal() {
            RoomEditorModal {
                server_id: modal_server_id.clone(),
                room: match modal {
                    RoomModal::Create => None,
                    RoomModal::Edit(room) => Some(room),
                },
                on_close: move |_| room_modal.set(None),
                on_saved: move |saved_room: ServerRoomSummary| {
                    let mut next_rooms = rooms();
                    upsert_room(&mut next_rooms, saved_room.clone());
                    next_rooms.sort_by_key(|room| room.position);
                    rooms.set(next_rooms);
                    active_room_id.set(Some(saved_room.id.clone()));
                    room_action_status.set(String::new());
                    on_state_change.call((
                        save_server_id.clone(),
                        ServerShellState {
                            chat_open: chat_open_for_room(&chat_open_by_room(), &saved_room.id),
                            room_kind: room_kind_attr(saved_room.kind),
                        },
                    ));
                },
            }
        }
    }
}

fn active_room(rooms: &[ServerRoomSummary], active_room_id: Option<&str>) -> Option<ActiveRoom> {
    let room = active_room_id
        .and_then(|room_id| rooms.iter().find(|room| room.id == room_id))
        .or_else(|| rooms.first())?;

    Some(ActiveRoom {
        id: room.id.clone(),
        kind: room.kind,
        name: room.name.clone(),
    })
}

fn chat_open_for_room(chat_open_by_room: &[(String, bool)], room_id: &str) -> bool {
    chat_open_by_room
        .iter()
        .find_map(|(saved_room_id, chat_open)| (saved_room_id == room_id).then_some(*chat_open))
        .unwrap_or(false)
}

fn upsert_room(rooms: &mut Vec<ServerRoomSummary>, room: ServerRoomSummary) {
    if let Some(saved_room) = rooms.iter_mut().find(|saved_room| saved_room.id == room.id) {
        *saved_room = room;
        return;
    }

    rooms.push(room);
}

fn room_icon(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "#",
        ServerRoomKind::Voice => "~",
        ServerRoomKind::TextAndVoice => "&",
    }
}

fn room_icon_class(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "w-3.5 shrink-0 text-center text-zinc-600",
        ServerRoomKind::Voice => {
            "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-zinc-500"
        }
        ServerRoomKind::TextAndVoice => {
            "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-accent"
        }
    }
}
