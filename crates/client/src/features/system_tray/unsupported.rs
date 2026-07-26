//! Запасная реализация системного трея для платформ без desktop-трея.

use dioxus::prelude::*;

/// Сообщает, поддерживается ли системный трей на текущей платформе.
pub(crate) const fn is_supported() -> bool {
    false
}

/// Оставляет системный трей выключенным на неподдерживаемых платформах.
#[component]
pub(crate) fn SystemTrayPlatformEffects(minimize_to_tray_on_close: Signal<bool>) -> Element {
    let _ = minimize_to_tray_on_close;
    rsx! {}
}
