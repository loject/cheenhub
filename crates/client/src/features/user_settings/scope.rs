//! User settings modal state scope.

use dioxus::prelude::*;

use super::page::{UserSettingsPage, UserSettingsSection};

/// Keeps user-settings UI state inside the user settings feature boundary.
#[component]
pub(crate) fn UserSettingsScope(on_close: EventHandler<()>) -> Element {
    let mut active_section = use_signal(|| UserSettingsSection::Profile);

    rsx! {
        UserSettingsPage {
            active_section: active_section(),
            on_select_section: move |section: UserSettingsSection| {
                active_section.set(section);
            },
            on_close,
        }
    }
}
