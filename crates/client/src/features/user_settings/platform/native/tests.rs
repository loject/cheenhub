use super::*;

#[test]
fn native_keeps_system_settings_available() {
    assert!(is_section_available(UserSettingsSection::System));
    assert!(resolve_section(UserSettingsSection::System) == UserSettingsSection::System);
}
