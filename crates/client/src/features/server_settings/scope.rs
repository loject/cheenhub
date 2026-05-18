//! Server settings feature state scope.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use super::page::{ServerSettingsPage, ServerSettingsSection};

/// Keeps server-settings UI state inside the settings feature boundary.
#[component]
pub(crate) fn ServerSettingsScope(
    server: ServerSummary,
    active: bool,
    on_server_updated: EventHandler<ServerSummary>,
    on_close: EventHandler<()>,
) -> Element {
    let mut active_section = use_signal(|| ServerSettingsSection::Overview);
    let wrapper_class = if active { "contents" } else { "hidden" };

    rsx! {
        div { class: wrapper_class,
            ServerSettingsPage {
                server,
                active_section: active_section(),
                on_select_section: move |section: ServerSettingsSection| {
                    active_section.set(section);
                },
                on_server_updated,
                on_close,
            }
        }
    }
}
