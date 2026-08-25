//! Desktop-закрытие основного окна после запуска обновления.

#[cfg(not(target_os = "linux"))]
use dioxus::desktop::{WindowCloseBehaviour, use_window};
use dioxus::prelude::*;

use crate::features::application_update::ApplicationUpdateShutdown;

/// Возвращает команду закрытия desktop-окна после запуска update-helper.
pub(crate) fn use_application_update_shutdown() -> ApplicationUpdateShutdown {
    #[cfg(target_os = "linux")]
    {
        ApplicationUpdateShutdown::new(|| {
            info!("keeping main application running while Linux update is installed");
        })
    }

    #[cfg(not(target_os = "linux"))]
    {
        let window = use_window();
        ApplicationUpdateShutdown::new(move || {
            window.set_close_behavior(WindowCloseBehaviour::WindowCloses);
            window.close();
            info!("closing main application window after update helper start");
        })
    }
}
