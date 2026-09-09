//! Выбор состава пользовательских настроек для текущей платформы.

#[cfg(feature = "web")]
#[path = "platform/web.rs"]
mod implementation;

#[cfg(not(feature = "web"))]
#[path = "platform/native.rs"]
mod implementation;

use super::page::UserSettingsSection;

/// Возвращает `true`, если раздел доступен на текущей платформе.
pub(super) fn is_section_available(section: UserSettingsSection) -> bool {
    implementation::is_section_available(section)
}

/// Возвращает доступный на текущей платформе раздел.
pub(super) fn resolve_section(section: UserSettingsSection) -> UserSettingsSection {
    implementation::resolve_section(section)
}
