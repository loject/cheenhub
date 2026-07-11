//! Main authenticated CheenHub application shell.

use cheenhub_contracts::rest::{ServerRoomKind, ServerSummary};
use dioxus::prelude::*;

use crate::Route;
use crate::features::app::active_room::ActiveRoomContext;
use crate::features::app::api;
use crate::features::app::workspace_route::AppWorkspaceRoute;
use crate::features::social::SocialPage;

use super::add_server_modal::AddServerModal;
use super::create_server_modal::CreateServerModal;
use super::empty_servers_panel::EmptyServersPanel;
use super::invite_link_modal::InviteLinkModal;
use super::server_instance::ServerInstance;
use super::server_rail::ServerRail;

#[derive(Clone, PartialEq)]
pub(crate) struct ActiveRoom {
    pub(crate) id: String,
    pub(crate) kind: ServerRoomKind,
    pub(crate) name: String,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ServerShellState {
    pub(crate) chat_open: bool,
    pub(crate) room_kind: &'static str,
}

#[derive(Clone, PartialEq)]
pub(crate) enum AppModal {
    InviteLink {
        server_id: String,
        server_name: String,
    },
}

/// Renders the static room UI shell.
#[component]
pub(crate) fn AppShell() -> Element {
    let navigator = use_navigator();
    let route = use_route::<Route>();
    let workspace = AppWorkspaceRoute::from_route(&route).unwrap_or(AppWorkspaceRoute::Friends);
    let route_active_server_id = workspace.server_id().map(ToOwned::to_owned);
    let selected_conversation_id = workspace.conversation_id().map(ToOwned::to_owned);
    let mut servers = use_signal(Vec::<ServerSummary>::new);
    let mut active_server_id = use_signal(|| route_active_server_id.clone());
    let mut loaded_servers = use_signal(|| false);
    let mut is_loading_servers = use_signal(|| false);
    let mut server_load_attempt = use_signal(|| 0_u32);
    let mut started_server_load_attempt = use_signal(|| None::<u32>);
    let mut retried_server_route = use_signal(|| None::<String>);
    let mut server_status = use_signal(String::new);
    let mut is_add_server_open = use_signal(|| false);
    let mut is_create_server_open = use_signal(|| false);
    let mut shell_state = use_signal(default_server_shell_state);
    let mut shell_state_by_server = use_signal(Vec::<(String, ServerShellState)>::new);
    let mut app_modal = use_signal(|| None::<AppModal>);
    let show_empty_servers = loaded_servers()
        && !is_loading_servers()
        && servers().is_empty()
        && server_status().is_empty();
    let social_workspace_active = workspace.is_social();
    let active_room = use_context::<ActiveRoomContext>();

    // Синхронизируем активную комнату и активный DM-диалог с маршрутом.
    let route_room_id = workspace.room_id().map(ToOwned::to_owned);
    let route_conversation_id = workspace.conversation_id().map(ToOwned::to_owned);
    use_effect(move || {
        active_room.set(route_room_id.clone());
        active_room.set_conversation_id(route_conversation_id.clone());
    });

    let route_active_server_id_for_sync = route_active_server_id.clone();
    use_effect(move || {
        if active_server_id() != route_active_server_id_for_sync {
            active_server_id.set(route_active_server_id_for_sync.clone());
        }
    });

    use_effect(move || {
        let attempt = server_load_attempt();
        if started_server_load_attempt() == Some(attempt) {
            return;
        }

        started_server_load_attempt.set(Some(attempt));
        loaded_servers.set(true);
        is_loading_servers.set(true);

        spawn(async move {
            debug!(attempt, "loading available app servers");
            match api::list_servers().await {
                Ok(next_servers) => {
                    info!(
                        attempt,
                        server_count = next_servers.len(),
                        "available app servers loaded"
                    );
                    servers.set(next_servers);
                    server_status.set(String::new());
                    retried_server_route.set(None);
                }
                Err(error) => {
                    warn!(
                        attempt,
                        %error,
                        "failed to load available app servers"
                    );
                    server_status.set(error);
                }
            }
            is_loading_servers.set(false);
        });
    });

    let route_active_server_id_for_retry = route_active_server_id.clone();
    use_effect(move || {
        let Some(server_id) = route_active_server_id_for_retry.clone() else {
            if retried_server_route().is_some() {
                retried_server_route.set(None);
            }
            return;
        };

        if is_loading_servers() || server_status().is_empty() {
            return;
        }

        if servers()
            .iter()
            .any(|server| server.id.as_str() == server_id.as_str())
        {
            return;
        }

        if retried_server_route().as_deref() == Some(server_id.as_str()) {
            return;
        }

        info!(
            %server_id,
            "retrying app server list load after opening server workspace"
        );
        retried_server_route.set(Some(server_id));
        server_load_attempt.with_mut(|attempt| *attempt = attempt.saturating_add(1));
    });

    use_effect(move || {
        if !loaded_servers() || is_loading_servers() {
            return;
        }

        if !server_status().is_empty() {
            return;
        }

        let Some(server_id) = active_server_id() else {
            return;
        };

        if servers()
            .iter()
            .any(|server| server.id.as_str() == server_id.as_str())
        {
            return;
        }

        warn!(
            %server_id,
            "saved app workspace references unavailable server; opening friends"
        );
        navigator.replace(Route::AppFriends {});
    });

    rsx! {
        main {
            id: "app-shell",
            "data-chat-open": if shell_state().chat_open { "true" } else { "false" },
            "data-room-kind": shell_state().room_kind,
            class: "grid-bg flex h-screen min-h-0 w-full overflow-hidden bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            ServerRail {
                servers: servers(),
                active_server_id: active_server_id(),
                social_active: social_workspace_active,
                is_loading: is_loading_servers(),
                status: server_status(),
                on_select_server: move |server_id: String| {
                    info!(%server_id, "switching app shell to server workspace");
                    let next_shell_state =
                        saved_server_shell_state(&shell_state_by_server(), &server_id)
                            .unwrap_or_else(default_server_shell_state);
                    // Маршрут обновляется асинхронно для уже смонтированной оболочки.
                    // Выбираем сервер до навигации, чтобы его рабочая область не осталась скрытой.
                    active_server_id.set(Some(server_id.clone()));
                    shell_state.set(next_shell_state);
                    navigator.push(Route::AppServer { server_id });
                },
                on_open_social: move |_| {
                    info!("switching app shell to social workspace");
                    navigator.push(Route::AppFriends {});
                },
                on_add_server: move |_| is_add_server_open.set(true),
            }
            if social_workspace_active {
                SocialPage {
                    selected_conversation_id,
                }
            }
            if !social_workspace_active && show_empty_servers {
                EmptyServersPanel {
                    on_create_server: move |_| is_add_server_open.set(true),
                }
            } else if !social_workspace_active && !show_empty_servers {
                for server in servers() {
                    ServerInstance {
                        key: "{server.id}",
                        active: !social_workspace_active
                            && active_server_id().as_deref() == Some(server.id.as_str()),
                        requested_room_id: if active_server_id().as_deref() == Some(server.id.as_str()) {
                            workspace.room_id().map(ToOwned::to_owned)
                        } else {
                            None
                        },
                        server,
                        on_state_change: move |(server_id, next_state): (String, ServerShellState)| {
                            let mut next_states = shell_state_by_server();
                            upsert_server_shell_state(&mut next_states, server_id.clone(), next_state);
                            shell_state_by_server.set(next_states);

                            if active_server_id().as_deref() == Some(server_id.as_str()) {
                                shell_state.set(next_state);
                            }
                        },
                        on_open_modal: move |modal: AppModal| app_modal.set(Some(modal)),
                        on_left_server: move |left_server_id: String| {
                            let mut next_servers = servers();
                            next_servers.retain(|server| server.id != left_server_id);

                            let mut next_states = shell_state_by_server();
                            next_states.retain(|(server_id, _)| server_id != &left_server_id);
                            shell_state_by_server.set(next_states.clone());

                            let next_active_server_id = active_server_id()
                                .as_ref()
                                .filter(|server_id| server_id.as_str() != left_server_id.as_str())
                                .cloned();
                            let next_shell_state = next_active_server_id
                                .as_deref()
                                .and_then(|server_id| {
                                    saved_server_shell_state(&next_states, server_id)
                                })
                                .unwrap_or_else(default_server_shell_state);

                            servers.set(next_servers);
                            shell_state.set(next_shell_state);
                            server_status.set(String::new());
                            if active_server_id().as_deref() == Some(left_server_id.as_str()) {
                                navigator.replace(Route::AppFriends {});
                            }
                        },
                        on_server_updated: move |server: ServerSummary| {
                            let mut next_servers = servers();
                            upsert_server_summary(&mut next_servers, server);
                            servers.set(next_servers);
                            server_status.set(String::new());
                        },
                    }
                }
            }
            if is_add_server_open() {
                AddServerModal {
                    on_close: move |_| is_add_server_open.set(false),
                    on_create_server: move |_| {
                        is_add_server_open.set(false);
                        is_create_server_open.set(true);
                    },
                    on_joined_server: move |server: ServerSummary| {
                        shell_state.set(default_server_shell_state());
                        let server_id = server.id.clone();
                        active_server_id.set(Some(server_id.clone()));
                        let mut next_servers = servers();
                        upsert_server_summary(&mut next_servers, server);
                        servers.set(next_servers);
                        server_status.set(String::new());
                        is_add_server_open.set(false);
                        navigator.push(Route::AppServer { server_id });
                    },
                }
            }
            if is_create_server_open() {
                CreateServerModal {
                    on_close: move |_| is_create_server_open.set(false),
                    on_created: move |server: ServerSummary| {
                        shell_state.set(default_server_shell_state());
                        let server_id = server.id.clone();
                        active_server_id.set(Some(server_id.clone()));
                        let mut next_servers = servers();
                        next_servers.push(server);
                        servers.set(next_servers);
                        server_status.set(String::new());
                        navigator.push(Route::AppServer { server_id });
                    },
                }
            }
            if let Some(AppModal::InviteLink { server_id, server_name }) = app_modal() {
                InviteLinkModal {
                    server_id,
                    server_name,
                    on_close: move |_| app_modal.set(None),
                }
            }
        }
    }
}

fn upsert_server_summary(servers: &mut Vec<ServerSummary>, server: ServerSummary) {
    if let Some(saved_server) = servers
        .iter_mut()
        .find(|saved_server| saved_server.id == server.id)
    {
        *saved_server = server;
        return;
    }

    servers.push(server);
}

fn default_server_shell_state() -> ServerShellState {
    ServerShellState {
        chat_open: false,
        room_kind: "text_and_voice",
    }
}

pub(crate) fn room_kind_attr(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "text",
        ServerRoomKind::Voice => "voice",
        ServerRoomKind::TextAndVoice => "text_and_voice",
    }
}

fn saved_server_shell_state(
    states: &[(String, ServerShellState)],
    server_id: &str,
) -> Option<ServerShellState> {
    states
        .iter()
        .find_map(|(saved_id, state)| (saved_id == server_id).then_some(*state))
}

fn upsert_server_shell_state(
    states: &mut Vec<(String, ServerShellState)>,
    server_id: String,
    state: ServerShellState,
) {
    if let Some((_, saved_state)) = states
        .iter_mut()
        .find(|(saved_id, _)| saved_id == &server_id)
    {
        *saved_state = state;
        return;
    }

    states.push((server_id, state));
}
