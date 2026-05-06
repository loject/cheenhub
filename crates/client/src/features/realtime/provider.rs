//! Dioxus realtime provider.

use dioxus::prelude::*;

use super::handle::create_handle;

/// Provides realtime context to authenticated app components.
#[component]
pub(crate) fn RealtimeProvider(children: Element) -> Element {
    let handle = use_signal(create_handle);
    let realtime = handle();
    use_context_provider(move || realtime.clone());

    rsx! {
        {children}
    }
}
