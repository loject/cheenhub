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
mod tests {
    use super::*;

    #[test]
    fn native_keeps_system_settings_available() {
        assert!(is_section_available(UserSettingsSection::System));
        assert!(resolve_section(UserSettingsSection::System) == UserSettingsSection::System);
    }
}
