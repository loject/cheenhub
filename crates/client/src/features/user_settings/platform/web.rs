//! Состав пользовательских настроек в браузере.

use super::super::page::UserSettingsSection;

/// Возвращает `true`, если раздел относится к браузерному клиенту.
pub(super) fn is_section_available(section: UserSettingsSection) -> bool {
    section != UserSettingsSection::System
}

/// Не позволяет браузерному клиенту открыть недоступный системный раздел.
pub(super) fn resolve_section(section: UserSettingsSection) -> UserSettingsSection {
    if is_section_available(section) {
        section
    } else {
        UserSettingsSection::Profile
    }
}

#[cfg(test)]
// Модуль подключается через `#[path = "platform/web.rs"]` из `platform.rs`, поэтому каталог
// дочернего модуля вычисляется относительно `user_settings/`. Без явного пути компилятор ищет
// `user_settings/tests.rs` вместо `user_settings/platform/web/tests.rs`.
#[path = "web/tests.rs"]
mod tests;
