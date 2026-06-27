//! Провайдер состояния системного трея.

use dioxus::prelude::*;

use super::SystemTrayHandle;
use super::load_minimize_to_tray_on_close;
use super::native::SystemTrayPlatformEffects;

/// Предоставляет настройки системного трея клиентскому приложению.
#[component]
pub(crate) fn SystemTrayProvider(children: Element) -> Element {
    let minimize_to_tray_on_close = use_signal(load_minimize_to_tray_on_close);
    let handle = SystemTrayHandle::new(minimize_to_tray_on_close);
    use_context_provider(move || handle.clone());

    rsx! {
        SystemTrayPlatformEffects { minimize_to_tray_on_close }
        {children}
    }
}
