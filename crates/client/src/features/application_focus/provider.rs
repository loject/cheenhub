//! Провайдер глобального состояния фокуса приложения.

use dioxus::prelude::*;

use crate::features::runtime::sleep_ms;

use super::application_is_focused;

/// Передаёт потомкам актуальное состояние фокуса окна приложения.
#[derive(Clone, Copy)]
pub(crate) struct ApplicationFocusContext {
    focused: Signal<bool>,
}

impl ApplicationFocusContext {
    /// Возвращает, находится ли приложение в фокусе.
    pub(crate) fn is_focused(&self) -> bool {
        (self.focused)()
    }
}

/// Предоставляет единый глобальный источник состояния фокуса приложения.
#[component]
pub(crate) fn ApplicationFocusProvider(children: Element) -> Element {
    let focus_state = use_signal(application_is_focused);
    use_context_provider(move || ApplicationFocusContext {
        focused: focus_state,
    });

    use_hook(move || spawn(track_application_focus(focus_state)));

    rsx! { {children} }
}

async fn track_application_focus(mut focus_state: Signal<bool>) {
    loop {
        let next_focused = application_is_focused();
        if focus_state() != next_focused {
            focus_state.set(next_focused);
            debug!(next_focused, "updated application focus state");
        }
        sleep_ms(250).await;
    }
}
