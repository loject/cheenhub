use super::*;

#[test]
fn browser_hides_and_rejects_system_settings() {
    assert!(!is_section_available(UserSettingsSection::System));
    assert!(resolve_section(UserSettingsSection::System) == UserSettingsSection::Profile);
    assert!(is_section_available(UserSettingsSection::Sound));
    assert!(resolve_section(UserSettingsSection::Sound) == UserSettingsSection::Sound);
}
