//! Per-server room state and workspace coordination.

use cheenhub_contracts::rest::{ServerRoomSummary, ServerSummary};
use dioxus::prelude::*;

use crate::Route;
use crate::features::app::active_room::ActiveRoomContext;
use crate::features::app::api;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::server_settings::ServerSettingsScope;
use crate::features::voice_chat::VoiceConnectionHandle;

use crate::features::app::server_permissions::ServerPermissionsContext;

use super::app_shell::{AppModal, ServerShellState, room_kind_attr};
use super::app_sidebar_footer::AppSidebarFooter;
use super::avatar::use_avatar_seed;
use super::room_editor_modal::RoomEditorModal;
use super::room_instance::RoomInstance;
use super::room_list_item::RoomListItem;
use super::server_context_menu::{ServerContextMenu, ServerMenuAction};
use super::server_rooms_action_error::ServerRoomsActionError;
use super::server_rooms_load_error::ServerRoomsLoadError;
use super::server_rooms_loading::ServerRoomsLoading;
use super::server_rooms_sidebar_styles as sidebar_styles;
use super::server_rooms_state::{
    RoomModal, ServerWorkspace, active_room, chat_open_for_room,
    clear_workspace_selection_if_needed, ensure_workspace_mounted, mount_workspace_if_missing,
    resolve_active_room_id, room_by_id, set_active_workspace_if_needed, upsert_room,
};

/// Owns room state for one server and renders the room sidebar and active room.
#[component]
pub(crate) fn ServerRoomsScope(
    server: ServerSummary,
    active: bool,
    requested_room_id: Option<String>,
    on_state_change: EventHandler<(String, ServerShellState)>,
    on_open_modal: EventHandler<AppModal>,
    on_left_server: EventHandler<String>,
    on_server_updated: EventHandler<ServerSummary>,
) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let navigator = use_navigator();
    let voice = use_context::<VoiceConnectionHandle>();
    let active_room_ctx = use_context::<ActiveRoomContext>();
    use_avatar_seed(current_user.id.clone());
    let mut rooms = use_signal(|| None::<Vec<ServerRoomSummary>>);
    let mut active_room_id = use_signal(|| None::<String>);
    let mut room_action_status = use_signal(String::new);
    let mut room_modal = use_signal(|| None::<RoomModal>);
    let mut is_server_menu_open = use_signal(|| false);
    let mut active_workspace = use_signal(|| None::<ServerWorkspace>);
    let mut mounted_workspaces = use_signal(Vec::<ServerWorkspace>::new);
    let mut mobile_workspace_open = use_signal(|| false);
    let mut voice_sidebar_loaded = use_signal(|| false);
    let mut reported_room_id = use_signal(|| None::<String>);
    let chat_open_by_room = use_signal(Vec::<(String, bool)>::new);
    let server_id = server.id.clone();
    let load_server_id = server.id.clone();
    let delete_server_id = server.id.clone();
    let select_server_id = server.id.clone();
    let save_server_id = server.id.clone();
    let invite_server_id = server.id.clone();
    let modal_server_id = server.id.clone();
    let open_settings_log_server_id = server.id.clone();
    let close_settings_log_server_id = server.id.clone();
    let server_name = server.name.clone();
    let invite_server_name = server_name.clone();
    let is_owner = server.is_owner;
    let server_permissions = ServerPermissionsContext::from_server(&server);
    let can_create_invite_links = server_permissions.can_create_invite_links;
    use_context_provider(move || server_permissions);
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
    let settings_workspace_active = matches!(active_workspace(), Some(ServerWorkspace::Settings));
    let sidebar_class = sidebar_styles::rooms_sidebar_class(settings_workspace_active);
    let sidebar_header_text_class =
        sidebar_styles::rooms_sidebar_header_text_class(settings_workspace_active);
    let sidebar_header_icon_class =
        sidebar_styles::rooms_sidebar_header_icon_class(settings_workspace_active);
    let room_section_title_class =
        sidebar_styles::room_section_title_class(settings_workspace_active);
    let voice_loader = voice.clone();
    let voice_load_server_id = server_id.clone();

    use_effect(move || {
        if rooms().is_some() {
            return;
        }

        let Some(Ok(next_rooms)) = room_load_resource.read().clone() else {
            return;
        };

        rooms.set(Some(next_rooms));
        if !voice_sidebar_loaded() {
            voice_sidebar_loaded.set(true);
            voice_loader.load_server_voice_rooms(voice_load_server_id.clone());
        }
    });

    let requested_room_id_for_sync = requested_room_id.clone();
    use_effect(move || {
        let Some(current_rooms) = rooms() else {
            return;
        };

        let next_active_room_id = resolve_active_room_id(
            &current_rooms,
            requested_room_id_for_sync.as_deref(),
            active_room_id().as_deref(),
        );
        if active_room_id() != next_active_room_id {
            active_room_id.set(next_active_room_id.clone());
        }

        // Обновляем глобальный контекст активной комнаты для фильтрации уведомлений.
        if active {
            active_room_ctx.set(next_active_room_id.clone());
        }

        let Some(room_id) = next_active_room_id else {
            clear_workspace_selection_if_needed(active_workspace, reported_room_id);
            return;
        };

        let workspace = ServerWorkspace::Room(room_id.clone());
        mount_workspace_if_missing(mounted_workspaces, workspace.clone());
        set_active_workspace_if_needed(active_workspace, workspace);

        if active && requested_room_id_for_sync.as_deref() != Some(room_id.as_str()) {
            info!(
                server_id = %server_id,
                room_id = %room_id,
                "replacing server workspace route with resolved room"
            );
            navigator.replace(Route::AppServerRoom {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
            });
        }

        if active
            && reported_room_id().as_deref() != Some(room_id.as_str())
            && let Some(room) = active_room(&current_rooms, Some(room_id.as_str()))
        {
            on_state_change.call((
                server_id.clone(),
                ServerShellState {
                    chat_open: chat_open_for_room(&chat_open_by_room(), &room.id),
                    room_kind: room_kind_attr(room.kind),
                },
            ));
            reported_room_id.set(Some(room_id));
        }
    });

    rsx! {
        aside {
            class: sidebar_class,
            "data-mobile-workspace-open": if mobile_workspace_open() { "true" } else { "false" },
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
                    span { class: sidebar_header_text_class,
                        span { class: "block text-[13px] font-semibold tracking-[-0.02em] text-zinc-100", "{server_name}" }
                        span { class: "mt-0.5 block text-[11px] text-zinc-500",
                            if is_owner { "Владелец сервера" } else { "Участник сервера" }
                        }
                    }
                    svg { class: sidebar_header_icon_class, fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
                    }
                }
                if is_server_menu_open() {
                    ServerContextMenu {
                        server_id: server.id.clone(),
                        is_owner,
                        can_open_settings: is_owner,
                        can_create_invite_links,
                        on_action: move |action: ServerMenuAction| {
                            is_server_menu_open.set(false);

                            match action {
                                ServerMenuAction::OpenSettings => {
                                    info!(
                                        server_id = %open_settings_log_server_id,
                                        "opened server settings workspace"
                                    );
                                    let workspace = ServerWorkspace::Settings;
                                    let mut next_mounted_workspaces = mounted_workspaces();
                                    ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
                                    mounted_workspaces.set(next_mounted_workspaces);
                                    active_workspace.set(Some(workspace));
                                    mobile_workspace_open.set(true);
                                }
                                ServerMenuAction::CreateInvite => {
                                    info!(
                                        server_id = %invite_server_id,
                                        "opened server invite modal from context menu"
                                    );
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
                    span { class: room_section_title_class, "Комнаты" }
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
                    ServerRoomsLoading {}
                } else if let Some(error) = initial_room_error.clone() {
                    ServerRoomsLoadError { message: error }
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
                            RoomListItem {
                                key: "{room.id}",
                                room: room.clone(),
                                is_active: matches!(active_workspace(), Some(ServerWorkspace::Room(ref id)) if id == &room.id),
                                is_owner,
                                voice_participants: voice.room_participants(&server.id, &room.id).unwrap_or_default(),
                                compact_when_settings_active: settings_workspace_active,
                                on_select: {
                                    let room = room.clone();
                                    let select_server_id = select_server_id.clone();
                                    move |_| {
                                        info!(
                                            server_id = %select_server_id,
                                            room_id = %room.id,
                                            room_kind = ?room.kind,
                                            "selected room workspace"
                                        );
                                        active_room_id.set(Some(room.id.clone()));
                                        let workspace = ServerWorkspace::Room(room.id.clone());
                                        let mut next_mounted_workspaces = mounted_workspaces();
                                        ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
                                        mounted_workspaces.set(next_mounted_workspaces);
                                        active_workspace.set(Some(workspace));
                                        mobile_workspace_open.set(true);
                                        if active {
                                            on_state_change.call((
                                                select_server_id.clone(),
                                                ServerShellState {
                                                    chat_open: chat_open_for_room(&chat_open_by_room(), &room.id),
                                                    room_kind: room_kind_attr(room.kind),
                                                },
                                            ));
                                            navigator.push(Route::AppServerRoom {
                                                server_id: select_server_id.clone(),
                                                room_id: room.id.clone(),
                                            });
                                        }
                                    }
                                },
                                on_edit: {
                                    let room = room.clone();
                                    move |_| room_modal.set(Some(RoomModal::Edit(room.clone())))
                                },
                                on_delete: {
                                    let room_id = room.id.clone();
                                    let delete_server_id = delete_server_id.clone();
                                    move |_| {
                                        room_action_status.set(String::new());
                                        let request_server_id = delete_server_id.clone();
                                        let state_server_id = delete_server_id.clone();
                                        let room_id = room_id.clone();
                                        spawn(async move {
                                            match api::delete_server_room(request_server_id, room_id.clone()).await {
                                                Ok(()) => {
                                                    let mut next_rooms = rooms().unwrap_or_default();
                                                    next_rooms.retain(|room| room.id != room_id);
                                                    let next_active_room_id = if active_room_id().as_deref() == Some(room_id.as_str()) {
                                                        next_rooms.first().map(|room| room.id.clone())
                                                    } else {
                                                        active_room_id()
                                                    };
                                                    active_room_id.set(next_active_room_id.clone());
                                                    rooms.set(Some(next_rooms.clone()));
                                                    let mut next_mounted_workspaces = mounted_workspaces();
                                                    next_mounted_workspaces.retain(|workspace| {
                                                        !matches!(workspace, ServerWorkspace::Room(id) if id == &room_id)
                                                    });
                                                    if matches!(active_workspace(), Some(ServerWorkspace::Room(ref id)) if id == &room_id) {
                                                        if let Some(next_room_id) = next_active_room_id.clone() {
                                                            ensure_workspace_mounted(&mut next_mounted_workspaces, ServerWorkspace::Room(next_room_id));
                                                        }
                                                        active_workspace.set(next_active_room_id.clone().map(ServerWorkspace::Room));
                                                    }
                                                    mounted_workspaces.set(next_mounted_workspaces);
                                                    if let Some(next_room) = active_room(&next_rooms, next_active_room_id.as_deref()) {
                                                        on_state_change.call((
                                                            state_server_id.clone(),
                                                            ServerShellState {
                                                                chat_open: chat_open_for_room(&chat_open_by_room(), &next_room.id),
                                                                room_kind: room_kind_attr(next_room.kind),
                                                            },
                                                        ));
                                                        navigator.replace(Route::AppServerRoom {
                                                            server_id: state_server_id.clone(),
                                                            room_id: next_room.id.clone(),
                                                        });
                                                    } else {
                                                        navigator.replace(Route::AppServer {
                                                            server_id: state_server_id.clone(),
                                                        });
                                                    }
                                                }
                                                Err(error) => {
                                                    room_action_status.set(error);
                                                }
                                            }
                                        });
                                    }
                                },
                            }
                        }
                    }
                }

                if !room_action_status().is_empty() {
                    ServerRoomsActionError { message: room_action_status() }
                }
            }
            AppSidebarFooter {
                realtime_label: server_name.clone(),
                settings_workspace_active,
                show_voice_controls: true,
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
                        mobile_workspace_open: mobile_workspace_open(),
                        chat_open_by_room,
                        on_state_change,
                        on_mobile_back: {
                            let mobile_back_server_id = server.id.clone();
                            move |_| {
                                if let Some(room_id) = active_room_id() {
                                    info!(
                                        server_id = %mobile_back_server_id,
                                        room_id = %room_id,
                                        "closed mobile room workspace"
                                    );
                                }
                                mobile_workspace_open.set(false);
                            }
                        },
                    }
                }
            }
        }
        if mounted_workspaces().contains(&ServerWorkspace::Settings) {
            ServerSettingsScope {
                key: "{server.id}:settings",
                server: server.clone(),
                active: active && matches!(active_workspace(), Some(ServerWorkspace::Settings)),
                on_server_updated,
                on_close: move |_| {
                    info!(
                        server_id = %close_settings_log_server_id,
                        "closed server settings workspace"
                    );
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
                    mobile_workspace_open.set(true);
                    room_action_status.set(String::new());
                    on_state_change.call((
                        save_server_id.clone(),
                        ServerShellState {
                            chat_open: chat_open_for_room(&chat_open_by_room(), &saved_room.id),
                            room_kind: room_kind_attr(saved_room.kind),
                        },
                    ));
                    navigator.push(Route::AppServerRoom {
                        server_id: save_server_id.clone(),
                        room_id: saved_room.id.clone(),
                    });
                },
            }
        }
    }
}
