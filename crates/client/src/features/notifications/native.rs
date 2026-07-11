//! Платформенная обертка для уведомлений.
//!
//! На веб-платформе делегирует в web-реализацию.
//! На нативных платформах предоставляет no-op stub.

#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::NotificationsProvider;

#[cfg(not(target_arch = "wasm32"))]
mod impl_ {
    use dioxus::prelude::*;

    /// На нативных платформах уведомления не поддерживаются.
    #[component]
    pub(crate) fn NotificationsProvider(children: Element) -> Element {
        rsx! {
            {children}
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use impl_::NotificationsProvider;
