use super::*;

#[test]
fn activation_requires_both_exact_values() {
    assert!(is_activation_pair(
        "developer_mode",
        "developer_mode@cheenhub.ru"
    ));
    for (nickname, email) in [
        ("Developer_Mode", "developer_mode@cheenhub.ru"),
        ("developer_mode", "developer_mode@ChinHub.ru"),
        ("developer_mode", "user@cheenhub.ru"),
        ("user", "developer_mode@cheenhub.ru"),
    ] {
        assert!(!is_activation_pair(nickname, email));
    }
}

#[test]
fn persisted_settings_round_trip_and_disabled_mode_ignores_overrides() {
    let settings = DeveloperSettings {
        enabled: true,
        force_websocket: true,
        ping_delay_ms: 1800,
    };
    let encoded = serde_json::to_string(&settings).unwrap();

    let restored: DeveloperSettings = serde_json::from_str(&encoded).unwrap();

    assert_eq!(restored, settings);
    assert!(restored.effective_force_websocket());
    assert_eq!(restored.effective_ping_delay_ms(), 1800);
    let disabled = DeveloperSettings {
        enabled: false,
        ..settings
    };
    assert!(!disabled.effective_force_websocket());
    assert_eq!(disabled.effective_ping_delay_ms(), 0);
    assert_eq!(
        DeveloperSettings::default(),
        DeveloperSettings {
            enabled: false,
            force_websocket: false,
            ping_delay_ms: 0
        }
    );
}
