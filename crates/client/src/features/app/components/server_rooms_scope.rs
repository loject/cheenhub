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
use super::room_delete_flow::render_room_delete_flow;
use super::room_editor_modal::RoomEditorModal;
use super::room_instance::RoomInstance;
use super::room_list_item::RoomListItem;
use super::server_context_menu::{ServerContextMenu, ServerMenuAction};
use super::server_menu_actions::{ServerMenuScope, apply_server_menu_action};
use super::server_room_workspace_sync::synchronize_room_workspace;
use super::server_rooms_action_error::ServerRoomsActionError;
use super::server_rooms_delete::RoomSidebarContext;
use super::server_rooms_empty_state::ServerRoomsEmptyState;
use super::server_rooms_load_error::ServerRoomsLoadError;
use super::server_rooms_loading::ServerRoomsLoading;
use super::server_rooms_menu_trigger::ServerRoomsMenuTrigger;
use super::server_rooms_save::apply_saved_room;
use super::server_rooms_sidebar_styles as sidebar_styles;
use super::server_rooms_state::{
    RoomModal, ServerWorkspace, active_room, chat_open_for_room,
    clear_workspace_selection_if_needed, close_server_settings_workspace, ensure_workspace_mounted,
    resolve_active_room_id, room_by_id,
};
use super::sidebar_menu_dismiss_layer::SidebarMenuDismissLayer;

/// Owns room state for one server and renders the room sidebar and active room.
#[component]
pub(crate) fn ServerRoomsScope(
    server: ServerSummary,
    active: bool,
    requested_room_id: Option<String>,
    on_state_change: EventHandler<(String, ServerShellState)>,
    on_open_modal: EventHandler<AppModal>,
    on_server_removed: EventHandler<String>,
    on_server_updated: EventHandler<ServerSummary>,
    on_open_user_settings: EventHandler<()>,
) -> Element {
    let current_user = use_context::<CurrentUserContext>().require_user();
    let navigator = use_navigator();
    let voice = use_context::<VoiceConnectionHandle>();
    let active_room_ctx = use_context::<ActiveRoomContext>();
    use_avatar_seed(current_user.id.clone());
    let mut active_room_id = use_signal(|| None::<String>);
    let mut room_action_status = use_signal(String::new);
    let mut room_modal = use_signal(|| None::<RoomModal>);
    let mut is_server_menu_open = use_signal(|| false);
    let mut is_profile_menu_open = use_signal(|| false);
    let mut is_connection_status_open = use_signal(|| false);
    let mut active_workspace = use_signal(|| None::<ServerWorkspace>);
    let mut mounted_workspaces = use_signal(Vec::<ServerWorkspace>::new);
    let mut mobile_workspace_open = use_signal(|| false);
    let mut reported_room_id = use_signal(|| None::<String>);
    let deleting_room_id = use_signal(|| None::<String>);
    let mut pending_delete_room = use_signal(|| None::<ServerRoomSummary>);
    let chat_open_by_room = use_signal(Vec::<(String, bool)>::new);
    let load_server_id = server.id.clone();
    let server_id = server.id.clone();
    let server_name = server.name.clone();
    let invite_server_name = server_name.clone();
    let is_owner = server.is_owner;
    let server_permissions = ServerPermissionsContext::from_server(&server);
    let can_create_invite_links = server_permissions.can_create_invite_links;
    let can_manage_rooms = server_permissions.can_manage_rooms;
    let room_roles = server.roles.clone();
    use_context_provider(move || server_permissions);
    let room_load_resource = use_resource(move || {
        let request_server_id = load_server_id.clone();
        let log_server_id = load_server_id.clone();
        async move {
            match api::list_server_rooms(request_server_id).await {
                Ok(rooms) => {
                    info!(
                        server_id = %log_server_id,
                        room_count = rooms.len(),
                        "loaded server rooms for sidebar"
                    );
                    Ok(rooms)
                }
                Err(error) => {
                    warn!(
                        server_id = %log_server_id,
                        %error,
                        "failed to load server rooms for sidebar"
                    );
                    Err(error)
                }
            }
        }
    });
    let room_load_result = room_load_resource.read().clone();
    let current_rooms = match &room_load_result {
        Some(Ok(rooms)) => rooms.clone(),
        _ => Vec::new(),
    };
    let has_loaded_rooms = matches!(room_load_result, Some(Ok(_)));
    let is_loading_rooms = !has_loaded_rooms
        && matches!(
            room_load_resource.state().cloned(),
            UseResourceState::Pending
        );
    let initial_room_error = match &room_load_result {
        Some(Err(error)) if !has_loaded_rooms => Some(error.clone()),
        _ => None,
    };
    // Повторная попытка отличима от первой загрузки: после restart() ресурс снова Pending,
    // но прежняя ошибка ещё доступна, поэтому блок ошибки должен показать busy-состояние.
    let is_retrying_rooms = !has_loaded_rooms
        && initial_room_error.is_some()
        && matches!(
            room_load_resource.state().cloned(),
            UseResourceState::Pending
        );
    let selected_room = active_room(&current_rooms, active_room_id().as_deref());
    let settings_workspace_active = matches!(active_workspace(), Some(ServerWorkspace::Settings));
    let sidebar_overlay_open =
        is_server_menu_open() || is_profile_menu_open() || is_connection_status_open();
    let sidebar_class =
        sidebar_styles::rooms_sidebar_class(settings_workspace_active, sidebar_overlay_open);
    let close_sidebar_overlay = use_callback(move |_| {
        is_server_menu_open.set(false);
        is_profile_menu_open.set(false);
        is_connection_status_open.set(false);
    });
    let sidebar_header_text_class =
        sidebar_styles::rooms_sidebar_header_text_class(settings_workspace_active);
    let sidebar_header_icon_class =
        sidebar_styles::rooms_sidebar_header_icon_class(settings_workspace_active);
    let room_section_title_class =
        sidebar_styles::room_section_title_class(settings_workspace_active);
    let voice_loader = voice.clone();
    let voice_load_server_id = server_id.clone();

    // Подписка на снимки голосовых комнат не зависит от загрузки списка комнат:
    // она нужна и после неудачного запроса, поэтому запускается один раз при монтировании.
    use_hook(move || {
        voice_loader.watch_server_voice_rooms(voice_load_server_id.clone());
    });

    let requested_room_id_for_sync = requested_room_id.clone();
    let sync_server_id = server_id.clone();
    use_effect(move || {
        let Some(Ok(current_rooms)) = room_load_resource.read().clone() else {
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

        synchronize_room_workspace(
            active,
            requested_room_id_for_sync.as_deref(),
            &sync_server_id,
            &room_id,
            &navigator,
            mounted_workspaces,
            active_workspace,
        );

        if active
            && reported_room_id().as_deref() != Some(room_id.as_str())
            && let Some(room) = active_room(&current_rooms, Some(room_id.as_str()))
        {
            on_state_change.call((
                sync_server_id.clone(),
                ServerShellState {
                    chat_open: chat_open_for_room(&chat_open_by_room(), &room.id),
                    room_kind: room_kind_attr(room.kind),
                },
            ));
            reported_room_id.set(Some(room_id));
        }
    });

    rsx! {
            if sidebar_overlay_open {
                SidebarMenuDismissLayer { on_close: move |_| close_sidebar_overlay.call(()) }
            }
            aside {
                class: sidebar_class,
                "data-mobile-workspace-open": if mobile_workspace_open() { "true" } else { "false" },
                onclick: move |_| close_sidebar_overlay.call(()),
                div { class: "relative border-b border-zinc-800/80 p-4",
                    ServerRoomsMenuTrigger {
                        server_name: server_name.clone(),
                        is_owner,
                        is_open: is_server_menu_open(),
                        text_class: sidebar_header_text_class,
                        icon_class: sidebar_header_icon_class,
                        on_toggle: move |_| is_server_menu_open.set(!is_server_menu_open()),
                    }
                    if is_server_menu_open() {
                        ServerContextMenu {
                            server_id: server.id.clone(),
                            server_name: server_name.clone(),
                            is_owner,
                            can_open_settings: is_owner,
                            can_create_invite_links,
                            on_action: {
                                let menu_server_id = server_id.clone();
                                let menu_invite_server_name = invite_server_name.clone();
                                let menu_scope = ServerMenuScope {
                                    is_menu_open: is_server_menu_open,
                                    mounted_workspaces,
                                    active_workspace,
                                    mobile_workspace_open,
                                    on_open_modal,
                                    on_server_removed,
                                };
                                move |action: ServerMenuAction| {
                                    apply_server_menu_action(
                                        action,
                                        menu_server_id.clone(),
                                        menu_invite_server_name.clone(),
                                        menu_scope,
                                    );
                                }
                            },
                        }
                    }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto p-3",
                    div { class: "mb-1.5 flex items-center justify-between px-1 text-[10px] font-medium uppercase tracking-[0.22em] text-zinc-600",
                        span { class: room_section_title_class, "Комнаты" }
                        if can_manage_rooms {
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
                        ServerRoomsLoadError {
                            message: error,
                            is_retrying: is_retrying_rooms,
                            on_retry: {
                                let mut resource = room_load_resource;
                                let retry_server_id = server_id.clone();
                                move |_| {
                                    info!(server_id = %retry_server_id, "retrying server rooms load");
                                    resource.restart();
                                }
                            },
                        }
                    } else if current_rooms.is_empty() {
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
                                    onclick: move |_| room_modal.set(Some(RoomModal::Create)),
                                    "Создать комнату"
                                }
                            }
                        }
                    } else {
                        div { class: "space-y-1",
                            for room in current_rooms.iter().cloned() {
                                RoomListItem {
                                    key: "{room.id}",
                                    room: room.clone(),
                                    is_active: matches!(active_workspace(), Some(ServerWorkspace::Room(ref id)) if id == &room.id),
                                    can_manage_rooms,
                                    is_deleting: deleting_room_id().as_deref() == Some(room.id.as_str()),
                                    voice_participants: voice.room_participants(&server_id, &room.id).unwrap_or_default(),
                                    compact_when_settings_active: settings_workspace_active,
                                    on_select: {
                                        let room = room.clone();
                                        let select_server_id = server_id.clone();
                                        move |_| {
                                            info!(
                                                server_id = %select_server_id,
                                                room_id = %room.id,
                                                room_kind = ?room.kind,
                                                "selected room workspace"
                                            );
                                            // Ошибка предыдущей операции относилась к другой комнате,
                                            // поэтому сбрасываем её при навигации пользователя.
                                            room_action_status.set(String::new());
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
                                        let room = room.clone();
                                        move |_| pending_delete_room.set(Some(room.clone()))
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
                    on_open_user_settings,
                    is_profile_menu_open,
                    is_connection_status_open,
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
                    on_close: {
                        let close_settings_server_id = server_id.clone();
                        move |_| {
                        info!(
                            server_id = %close_settings_server_id,
                            "closed server settings workspace"
                        );
                        close_server_settings_workspace(
                            active_room_id(),
                            mounted_workspaces,
                            active_workspace,
                        );
                        }
                    },
                }
            }
            // Пустое состояние не показываем, пока список не загружен: при ошибке запроса
    // пользователь должен видеть блок ошибки с повторной попыткой, а не «Комнат пока нет».
    if !is_loading_rooms
                && initial_room_error.is_none()
                && selected_room.is_none()
                && !matches!(active_workspace(), Some(ServerWorkspace::Settings))
            {
                ServerRoomsEmptyState {
                    can_manage_rooms,
                    on_create: move |_| room_modal.set(Some(RoomModal::Create)),
                }
            }
            if let Some(modal) = room_modal() {
                RoomEditorModal {
                    server_id: server_id.clone(),
                    room: match modal {
                        RoomModal::Create => None,
                        RoomModal::Edit(room) => Some(room),
                    },
                    roles: room_roles.clone(),
                    on_close: move |_| room_modal.set(None),
                    on_saved: {
                        let save_server_id = server_id.clone();
                        move |saved_room: ServerRoomSummary| {
                            apply_saved_room(
                                save_server_id.clone(),
                                saved_room,
                                RoomSidebarContext {
                                    rooms: room_load_resource,
                                    active_room_id,
                                    mounted_workspaces,
                                    active_workspace,
                                    chat_open_by_room,
                                    on_state_change,
                                },
                                mobile_workspace_open,
                                room_action_status,
                                navigator,
                            );
                        }
                    },
                }
            }
            if let Some(room_to_delete) = pending_delete_room() {
                {render_room_delete_flow(
                    room_to_delete,
                    server_id.clone(),
                    pending_delete_room,
                    deleting_room_id,
                    room_action_status,
                    RoomSidebarContext {
                        rooms: room_load_resource,
                        active_room_id,
                        mounted_workspaces,
                        active_workspace,
                        chat_open_by_room,
                        on_state_change,
                    },
                )}
            }
        }
}
