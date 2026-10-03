//! Проверки доменных данных настроек хоста.

use super::{HostEmailSettings, LogLevel};

#[test]
fn log_level_round_trips_through_database_value() {
    for level in [
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
        LogLevel::Trace,
    ] {
        assert_eq!(LogLevel::parse(level.as_str()).ok(), Some(level));
    }
}

#[test]
fn unknown_log_level_is_rejected_instead_of_silently_downgraded() {
    let parsed = LogLevel::parse("verbose");

    assert!(parsed.is_err());
}

#[test]
fn database_gmail_credentials_override_environment_fallback() {
    let settings = HostEmailSettings {
        gmail_client_id: Some("database-id".to_owned()),
        gmail_client_secret: None,
        ..HostEmailSettings::default()
    }
    .with_gmail_oauth_fallback(
        Some("environment-id".to_owned()),
        Some("environment-secret".to_owned()),
    );

    assert_eq!(settings.gmail_client_id.as_deref(), Some("database-id"));
    assert_eq!(
        settings.gmail_client_secret.as_deref(),
        Some("environment-secret")
    );
}
