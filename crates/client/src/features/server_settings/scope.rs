//! Server settings feature state scope.

use dioxus::prelude::*;

use super::page::{ServerSettingsPage, ServerSettingsSection};

/// Keeps server-settings UI state inside the settings feature boundary.
#[component]
pub(crate) fn ServerSettingsScope(
    server_id: String,
    server_name: String,
    active: bool,
    on_close: EventHandler<()>,
) -> Element {
    let mut active_section = use_signal(|| ServerSettingsSection::Overview);
    let wrapper_class = if active { "contents" } else { "hidden" };

    rsx! {
        div { class: wrapper_class,
            ServerSettingsPage {
                server_id,
                server_name,
                active_section: active_section(),
                on_select_section: move |section: ServerSettingsSection| {
                    active_section.set(section);
                },
                on_close,
            }
        }
    }
}
