//! Заглушка закрытия основного окна для платформ без desktop-окна.

use dioxus::prelude::*;

use crate::features::application_update::ApplicationUpdateShutdown;

/// Возвращает no-op команду для неподдерживаемой платформы.
pub(crate) fn use_application_update_shutdown() -> ApplicationUpdateShutdown {
    ApplicationUpdateShutdown::new(|| {
        debug!("application update shutdown requested on unsupported build");
    })
}
