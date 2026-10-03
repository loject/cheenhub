//! Проверки доменных данных настроек хоста.

use super::HostEmailSettings;

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
