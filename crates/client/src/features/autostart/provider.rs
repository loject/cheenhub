//! Провайдер состояния автоматического запуска.

use dioxus::prelude::*;

use super::AutostartHandle;
use super::native;

/// Предоставляет настройку автоматического запуска дочерним компонентам.
#[component]
pub(crate) fn AutostartProvider(children: Element) -> Element {
    let (initial_enabled, initial_error) = use_hook(|| match native::is_enabled() {
        Ok(enabled) => {
            info!(enabled, "loaded CheenHub autostart registration state");
            (enabled, None)
        }
        Err(message) => {
            warn!(error = %message, "failed to load CheenHub autostart registration state");
            (false, Some(message))
        }
    });
    let enabled = use_signal(|| initial_enabled);
    let error = use_signal(|| initial_error);
    let handle = AutostartHandle::new(enabled, error);
    use_context_provider(move || handle.clone());

    rsx! { {children} }
}
