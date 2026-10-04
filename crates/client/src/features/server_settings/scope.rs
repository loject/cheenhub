//! Server settings feature state scope.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use super::deletion_modal::ServerDeletionModal;
use super::page::{ServerSettingsPage, ServerSettingsSection};

/// Keeps server-settings UI state inside the settings feature boundary.
#[component]
pub(crate) fn ServerSettingsScope(
    server: ServerSummary,
    active: bool,
    on_server_updated: EventHandler<ServerSummary>,
    on_close: EventHandler<()>,
    /// Сообщает владельцу рабочей области об успешном удалении сервера.
    on_server_deleted: EventHandler<String>,
) -> Element {
    let mut delete_open = use_signal(|| false);
    let mut active_section = use_signal(|| ServerSettingsSection::Overview);
    let delete_server_id = server.id.clone();
    let is_owner = server.is_owner;
    let wrapper_class = if active { "contents" } else { "hidden" };

    rsx! {
        div { class: wrapper_class,
            ServerSettingsPage {
                server: server.clone(),
                on_delete_request: move |_| {
                    if is_owner {
                        info!(server_id = %delete_server_id, "opened server deletion confirmation from settings");
                        delete_open.set(true);
                    }
                },
                active_section: active_section(),
                on_select_section: move |section: ServerSettingsSection| {
                    active_section.set(section);
                },
                on_server_updated,
                on_close,
            }
            if active && delete_open() {
                ServerDeletionModal {
                    server,
                    on_close: move |_| delete_open.set(false),
                    on_deleted: move |id| {
                        delete_open.set(false);
                        on_server_deleted.call(id);
                    },
                }
            }
        }
    }
}
