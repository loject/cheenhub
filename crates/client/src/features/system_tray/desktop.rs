//! Desktop-реализация системного трея.

use dioxus::desktop::{
    WindowCloseBehaviour, icon_from_memory, trayicon, use_tray_menu_event_handler, use_window,
};
use dioxus::prelude::*;

const OPEN_MENU_ID: &str = "cheenhub.system_tray.open";
const QUIT_MENU_ID: &str = "cheenhub.system_tray.quit";

/// Подключает desktop-трей и синхронизирует поведение закрытия окна.
#[component]
pub(crate) fn SystemTrayPlatformEffects(minimize_to_tray_on_close: Signal<bool>) -> Element {
    let window = use_window();
    let tray_window = window.clone();
    let _tray_icon = use_hook(|| {
        let tray_icon = trayicon::init_tray_icon(
            build_tray_menu(),
            icon_from_memory(include_bytes!(
                "../../../../../crates/client/public/icons/icon-512.png"
            ))
            .ok(),
        );
        if let Err(error) = tray_icon.set_tooltip(Some("CheenHub")) {
            warn!(%error, "failed to set system tray tooltip");
        }
        info!("initialized CheenHub system tray icon");
        tray_icon
    });

    use_effect(move || {
        let enabled = minimize_to_tray_on_close();
        let behaviour = if enabled {
            WindowCloseBehaviour::WindowHides
        } else {
            WindowCloseBehaviour::WindowCloses
        };
        window.set_close_behavior(behaviour);
        info!(
            enabled,
            "updated desktop close behavior from system tray preference"
        );
    });

    use_tray_menu_event_handler(move |event| match event.id().as_ref() {
        OPEN_MENU_ID => {
            tray_window.set_visible(true);
            tray_window.set_focus();
            info!("opened CheenHub window from system tray menu");
        }
        QUIT_MENU_ID => {
            tray_window.set_close_behavior(WindowCloseBehaviour::WindowCloses);
            tray_window.close();
            info!("closing CheenHub from system tray menu");
        }
        _ => {}
    });

    rsx! {}
}

fn build_tray_menu() -> trayicon::DioxusTrayMenu {
    use trayicon::menu::{Menu, MenuItem, PredefinedMenuItem};

    let open = MenuItem::with_id(OPEN_MENU_ID, "Открыть CheenHub", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit = MenuItem::with_id(QUIT_MENU_ID, "Выйти", true, None);
    let menu = Menu::new();
    menu.append_items(&[&open, &separator, &quit])
        .expect("failed to build CheenHub tray menu");
    menu
}
