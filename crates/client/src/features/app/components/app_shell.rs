//! Main authenticated CheenHub application shell.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use crate::features::app::api;

use super::add_server_modal::AddServerModal;
use super::create_server_modal::CreateServerModal;
use super::empty_servers_panel::EmptyServersPanel;
use super::invite_link_modal::InviteLinkModal;
use super::server_instance::ServerInstance;
use super::server_rail::ServerRail;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ActiveRoom {
    pub(crate) kind: &'static str,
    pub(crate) name: &'static str,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ServerShellState {
    pub(crate) chat_open: bool,
    pub(crate) room_kind: &'static str,
}

#[derive(Clone, PartialEq)]
pub(crate) enum AppModal {
    InviteLink { server_name: String },
}

/// Renders the static room UI shell.
#[component]
pub(crate) fn AppShell() -> Element {
    let mut servers = use_signal(Vec::<ServerSummary>::new);
    let mut active_server_id = use_signal(|| None::<String>);
    let mut loaded_servers = use_signal(|| false);
    let mut is_loading_servers = use_signal(|| false);
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

    use_effect(move || {
        if loaded_servers() {
            return;
        }
        loaded_servers.set(true);
        is_loading_servers.set(true);

        spawn(async move {
            match api::list_servers().await {
                Ok(next_servers) => {
                    if active_server_id().is_none() {
                        active_server_id.set(next_servers.first().map(|server| server.id.clone()));
                    }
                    servers.set(next_servers);
                    server_status.set(String::new());
                }
                Err(error) => {
                    server_status.set(error);
                }
            }
            is_loading_servers.set(false);
        });
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
                is_loading: is_loading_servers(),
                status: server_status(),
                on_select_server: move |server_id: String| {
                    let next_shell_state =
                        saved_server_shell_state(&shell_state_by_server(), &server_id)
                            .unwrap_or_else(default_server_shell_state);
                    active_server_id.set(Some(server_id));
                    shell_state.set(next_shell_state);
                },
                on_add_server: move |_| is_add_server_open.set(true),
            }
            if show_empty_servers {
                EmptyServersPanel {
                    on_create_server: move |_| is_add_server_open.set(true),
                }
            } else {
                for server in servers() {
                    ServerInstance {
                        key: "{server.id}",
                        active: active_server_id().as_deref() == Some(server.id.as_str()),
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
                }
            }
            if is_create_server_open() {
                CreateServerModal {
                    on_close: move |_| is_create_server_open.set(false),
                    on_created: move |server: ServerSummary| {
                        shell_state.set(default_server_shell_state());
                        active_server_id.set(Some(server.id.clone()));
                        let mut next_servers = servers();
                        next_servers.push(server);
                        servers.set(next_servers);
                        server_status.set(String::new());
                    },
                }
            }
            if let Some(AppModal::InviteLink { server_name }) = app_modal() {
                InviteLinkModal {
                    server_name,
                    on_close: move |_| app_modal.set(None),
                }
            }
        }
    }
}

fn default_server_shell_state() -> ServerShellState {
    ServerShellState {
        chat_open: false,
        room_kind: "mixed",
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
