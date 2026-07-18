//! Заглушка единственного экземпляра для неподдерживаемых платформ.

use dioxus::prelude::*;

/// Разрешает обычный запуск на неподдерживаемой платформе.
pub(crate) fn prepare() -> Result<bool, String> {
    Ok(true)
}

/// Не создаёт UI-эффекты на неподдерживаемой платформе.
#[component]
pub(crate) fn SingleInstanceEffects() -> Element {
    rsx! {}
}
