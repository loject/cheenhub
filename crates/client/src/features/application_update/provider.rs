//! Провайдер пользовательского состояния обновлений приложения.

use dioxus::prelude::*;

use super::effects::ApplicationUpdateEffects;
use super::handle::{ApplicationUpdateHandle, ApplicationUpdateState};
use crate::features::toast::ToastProvider;

/// Предоставляет состояние проверки обновлений всем экранам клиента.
#[component]
pub(crate) fn ApplicationUpdateProvider(children: Element) -> Element {
    let state = use_signal(ApplicationUpdateState::default);
    let handle = ApplicationUpdateHandle::new(state);
    use_context_provider(move || handle);

    rsx! {
        ToastProvider {
            ApplicationUpdateEffects {
                {children}
            }
        }
    }
}
