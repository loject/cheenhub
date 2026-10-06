//! Состав пользовательских настроек в native-клиенте.

use super::super::page::UserSettingsSection;

/// Оставляет доступными все разделы native-клиента.
pub(super) fn is_section_available(_section: UserSettingsSection) -> bool {
    true
}

/// Сохраняет выбранный раздел native-клиента без изменений.
pub(super) fn resolve_section(section: UserSettingsSection) -> UserSettingsSection {
    section
}

#[cfg(test)]
// Модуль подключается через `#[path = "platform/native.rs"]` из `platform.rs`, поэтому каталог
// дочернего модуля вычисляется относительно `user_settings/`. Без явного пути компилятор ищет
// `user_settings/tests.rs` вместо `user_settings/platform/native/tests.rs`.
#[path = "native/tests.rs"]
mod tests;
