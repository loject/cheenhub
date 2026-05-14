//! Per-server room state and workspace coordination.

use cheenhub_contracts::rest::{ServerRoomSummary, ServerSummary};
use dioxus::prelude::*;

use crate::features::app::api;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::network::RealtimeConnectionStatusIndicator;
use crate::features::server_settings::ServerSettingsScope;
use crate::features::user_settings::UserSettingsScope;
use crate::features::voice_chat::SidebarVoiceControls;

use super::app_shell::{AppModal, ServerShellState, room_kind_attr};
use super::avatar::UserAvatar;
use super::room_editor_modal::RoomEditorModal;
use super::room_instance::RoomInstance;
use super::server_context_menu::{ServerContextMenu, ServerMenuAction};
use super::server_rooms_state::{
    ServerWorkspace, active_room, chat_open_for_room, ensure_workspace_mounted, room_by_id,
    room_icon, room_icon_class, upsert_room,
};

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
    let current_user = use_context::<CurrentUserContext>().require_user();
    let mut rooms = use_signal(|| None::<Vec<ServerRoomSummary>>);
    let mut active_room_id = use_signal(|| None::<String>);
    let mut room_action_status = use_signal(String::new);
    let mut room_modal = use_signal(|| None::<RoomModal>);
    let mut is_server_menu_open = use_signal(|| false);
    let mut is_user_settings_open = use_signal(|| false);
    let mut active_workspace = use_signal(|| None::<ServerWorkspace>);
    let mut mounted_workspaces = use_signal(Vec::<ServerWorkspace>::new);
    let chat_open_by_room = use_signal(Vec::<(String, bool)>::new);
    let server_id = server.id.clone();
    let load_server_id = server.id.clone();
    let delete_server_id = server.id.clone();
    let select_server_id = server.id.clone();
    let save_server_id = server.id.clone();
    let invite_server_id = server.id.clone();
    let modal_server_id = server.id.clone();
    let server_name = server.name.clone();
    let invite_server_name = server_name.clone();
    let is_owner = server.is_owner;
    let room_load_resource = use_resource(move || {
        let request_server_id = load_server_id.clone();

        async move { api::list_server_rooms(request_server_id).await }
    });
    let room_load_result = room_load_resource.read().clone();
    let current_rooms = rooms().unwrap_or_default();
    let is_loading_rooms = rooms().is_none() && room_load_result.is_none();
    let initial_room_error = match room_load_result {
        Some(Err(error)) if rooms().is_none() => Some(error),
        _ => None,
    };
    let selected_room = active_room(&current_rooms, active_room_id().as_deref());
    use_effect(move || {
        if rooms().is_some() {
            return;
        }

        let Some(Ok(next_rooms)) = room_load_resource.read().clone() else {
            return;
        };

        let next_active_room_id = next_rooms.first().map(|room| room.id.clone());
        active_room_id.set(next_active_room_id.clone());
        rooms.set(Some(next_rooms.clone()));

        if let Some(room_id) = next_active_room_id.clone() {
            let workspace = ServerWorkspace::Room(room_id);
            let mut next_mounted_workspaces = mounted_workspaces();
            ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
            mounted_workspaces.set(next_mounted_workspaces);
            active_workspace.set(Some(workspace));
        }

        if let Some(room_id) = next_active_room_id
            && let Some(room) = active_room(&next_rooms, Some(room_id.as_str()))
        {
            on_state_change.call((
                server_id.clone(),
                ServerShellState {
                    chat_open: chat_open_for_room(&chat_open_by_room(), &room.id),
                    room_kind: room_kind_attr(room.kind),
                },
            ));
        }
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
                        on_action: move |action: ServerMenuAction| {
                            is_server_menu_open.set(false);

                            match action {
                                ServerMenuAction::OpenSettings => {
                                    let workspace = ServerWorkspace::Settings;
                                    let mut next_mounted_workspaces = mounted_workspaces();
                                    ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
                                    mounted_workspaces.set(next_mounted_workspaces);
                                    active_workspace.set(Some(workspace));
                                }
                                ServerMenuAction::CreateInvite => {
                                    on_open_modal.call(AppModal::InviteLink {
                                        server_id: invite_server_id.clone(),
                                        server_name: invite_server_name.clone(),
                                    });
                                }
                                ServerMenuAction::LeftServer(server_id) => {
                                    on_left_server.call(server_id);
                                }
                            }
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

                if is_loading_rooms {
                    div { class: "space-y-2 px-1 py-2",
                        div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/80" }
                        div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/60" }
                        div { class: "h-9 animate-pulse rounded-lg bg-zinc-900/40" }
                    }
                } else if let Some(error) = initial_room_error.clone() {
                    div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{error}"
                    }
                } else if current_rooms.is_empty() {
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
                        for room in current_rooms.clone() {
                            div {
                                key: "{room.id}",
                                "data-active": if matches!(active_workspace(), Some(ServerWorkspace::Room(active_room_id)) if active_room_id == room.id) { "true" } else { "false" },
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
                                            let workspace = ServerWorkspace::Room(room.id.clone());
                                            let mut next_mounted_workspaces = mounted_workspaces();
                                            ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
                                            mounted_workspaces.set(next_mounted_workspaces);
                                            active_workspace.set(Some(workspace));

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
                                                                let mut next_rooms =
                                                                    rooms().unwrap_or_default();
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
                                                                rooms.set(Some(next_rooms.clone()));
                                                                let mut next_mounted_workspaces =
                                                                    mounted_workspaces();
                                                                next_mounted_workspaces.retain(
                                                                    |workspace| {
                                                                        !matches!(
                                                                            workspace,
                                                                            ServerWorkspace::Room(
                                                                                mounted_room_id,
                                                                            ) if mounted_room_id == &room_id
                                                                        )
                                                                    },
                                                                );
                                                                if matches!(
                                                                    active_workspace(),
                                                                    Some(ServerWorkspace::Room(
                                                                        active_room_id,
                                                                    )) if active_room_id == room_id
                                                                ) {
                                                                    if let Some(next_room_id) =
                                                                        next_active_room_id.clone()
                                                                    {
                                                                        ensure_workspace_mounted(
                                                                            &mut next_mounted_workspaces,
                                                                            ServerWorkspace::Room(
                                                                                next_room_id,
                                                                            ),
                                                                        );
                                                                    }
                                                                    active_workspace.set(
                                                                        next_active_room_id
                                                                            .clone()
                                                                            .map(ServerWorkspace::Room),
                                                                    );
                                                                }
                                                                mounted_workspaces
                                                                    .set(next_mounted_workspaces);

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
                div { class: "mb-2 flex items-center gap-2 rounded-[16px] border border-zinc-800 bg-zinc-900/70 px-3 py-2",
                    RealtimeConnectionStatusIndicator {}
                    div { class: "min-w-0 flex-1",
                        div { class: "truncate text-[11px] font-medium text-zinc-100", "{server_name}" }
                        div { class: "truncate text-[11px] text-zinc-500", "realtime соединение" }
                    }
                }
                SidebarVoiceControls {}
                div { class: "flex items-center gap-3 rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5",
                    UserAvatar {
                        nickname: current_user.nickname.clone(),
                        avatar_url: current_user.avatar_url.clone(),
                        class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-accent text-[12px] font-bold text-white".to_owned(),
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "truncate text-[12px] font-medium text-zinc-100", "{current_user.nickname}" }
                        div { class: "truncate text-[11px] text-zinc-500", "в приложении" }
                    }
                    button {
                        r#type: "button",
                        class: "rounded-lg p-2 text-zinc-500 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-zinc-800 hover:text-zinc-200",
                        "aria-label": "Настройки пользователя",
                        onclick: move |_| is_user_settings_open.set(true),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.075.04.149.083.22.127.324.2.72.226 1.075.091l1.217-.462a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.82c-.293.24-.438.613-.431.992a7.723 7.723 0 0 1 0 .255c-.007.379.138.752.431.992l1.003.82c.433.354.54.972.26 1.431l-1.296 2.247a1.125 1.125 0 0 1-1.37.49l-1.217-.462c-.355-.135-.751-.109-1.076.091a6.72 6.72 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.542-.56.94-1.11.94h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.063-.374-.313-.686-.645-.87a6.52 6.52 0 0 1-.22-.127c-.324-.2-.72-.226-1.075-.091l-1.217.462a1.125 1.125 0 0 1-1.37-.49l-1.296-2.247a1.125 1.125 0 0 1 .26-1.431l1.003-.82c.293-.24.438-.613.431-.992a6.932 6.932 0 0 1 0-.255c.007-.379-.138-.752-.431-.992l-1.003-.82a1.125 1.125 0 0 1-.26-1.431l1.296-2.247a1.125 1.125 0 0 1 1.37-.49l1.217.462c.355-.135.751-.109 1.076-.091.071-.044.145-.087.220-.128.331-.183.581-.495.644-.869l.213-1.281Z" }
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" }
                        }
                    }
                }
            }
        }

        for workspace in mounted_workspaces() {
            if let ServerWorkspace::Room(room_id) = workspace {
                if let Some(room) = room_by_id(&current_rooms, &room_id) {
                    RoomInstance {
                        key: "{server.id}:{room.id}",
                        server_id: server.id.clone(),
                        room: room.clone(),
                        active: active && matches!(active_workspace(), Some(ServerWorkspace::Room(active_room_id)) if active_room_id == room.id),
                        chat_open_by_room,
                        on_state_change,
                    }
                }
            }
        }
        if mounted_workspaces().contains(&ServerWorkspace::Settings) {
            ServerSettingsScope {
                key: "{server.id}:settings",
                server_name: server.name.clone(),
                active: active && matches!(active_workspace(), Some(ServerWorkspace::Settings)),
                on_close: move |_| {
                    if let Some(room_id) = active_room_id() {
                        let workspace = ServerWorkspace::Room(room_id);
                        let mut next_mounted_workspaces = mounted_workspaces();
                        ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
                        mounted_workspaces.set(next_mounted_workspaces);
                        active_workspace.set(Some(workspace));
                    } else {
                        active_workspace.set(None);
                    }
                },
            }
        }
        if !is_loading_rooms && selected_room.is_none() && !matches!(active_workspace(), Some(ServerWorkspace::Settings)) {
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
                    let mut next_rooms = rooms().unwrap_or_default();
                    upsert_room(&mut next_rooms, saved_room.clone());
                    next_rooms.sort_by_key(|room| room.position);
                    rooms.set(Some(next_rooms));
                    active_room_id.set(Some(saved_room.id.clone()));
                    let workspace = ServerWorkspace::Room(saved_room.id.clone());
                    let mut next_mounted_workspaces = mounted_workspaces();
                    ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
                    mounted_workspaces.set(next_mounted_workspaces);
                    active_workspace.set(Some(workspace));
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
        if is_user_settings_open() {
            UserSettingsScope {
                on_close: move |_| is_user_settings_open.set(false),
            }
        }
    }
}
