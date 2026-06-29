//! Desktop-закрытие основного окна после запуска обновления.

use dioxus::desktop::{WindowCloseBehaviour, use_window};
use dioxus::prelude::*;

use crate::features::application_update::ApplicationUpdateShutdown;

/// Возвращает команду закрытия desktop-окна после запуска update-helper.
pub(crate) fn use_application_update_shutdown() -> ApplicationUpdateShutdown {
    let window = use_window();
    ApplicationUpdateShutdown::new(move || {
        window.set_close_behavior(WindowCloseBehaviour::WindowCloses);
        window.close();
        info!("closing main application window after update helper start");
    })
}
