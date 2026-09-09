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
mod tests {
    use super::*;

    #[test]
    fn browser_hides_and_rejects_system_settings() {
        assert!(!is_section_available(UserSettingsSection::System));
        assert!(resolve_section(UserSettingsSection::System) == UserSettingsSection::Profile);
        assert!(is_section_available(UserSettingsSection::Sound));
        assert!(resolve_section(UserSettingsSection::Sound) == UserSettingsSection::Sound);
    }
}
