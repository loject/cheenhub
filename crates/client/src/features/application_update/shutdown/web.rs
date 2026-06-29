//! Web-заглушка закрытия основного окна после запуска обновления.

use dioxus::prelude::*;

use crate::features::application_update::ApplicationUpdateShutdown;

/// Возвращает no-op команду для web-сборки.
pub(crate) fn use_application_update_shutdown() -> ApplicationUpdateShutdown {
    ApplicationUpdateShutdown::new(|| {
        debug!("application update shutdown requested on web build");
    })
}
