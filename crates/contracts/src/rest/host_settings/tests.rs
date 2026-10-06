use super::{EmailTransport, HostEmailSettingsResponse, HostMetricsSample};

#[test]
fn deserializes_metrics_sample_without_disk_field_from_old_proxy() {
    let json = r#"{
        "sampled_at_unix_ms": 1,
        "cpu": {"system_percent": 1.0, "cheenhub_percent": 2.0, "database_percent": 3.0, "other_percent": 4.0, "logical_processors_percent": []},
        "memory": {"total_bytes": 1, "used_bytes": 2, "cheenhub_bytes": 3, "database_bytes": 4, "other_bytes": 5},
        "network": {"sent_bytes_per_second": 1.0, "received_bytes_per_second": 2.0, "sent_bytes_total": 3, "received_bytes_total": 4}
    }"#;

    let sample: HostMetricsSample =
        serde_json::from_str(json).expect("old metrics sample deserializes");

    assert!(sample.disk.is_none());
}

#[test]
fn email_settings_response_contains_only_secret_presence_flags() {
    let response = HostEmailSettingsResponse {
        transport: EmailTransport::GmailApi,
        email_send_timeout_seconds: 10,
        smtp_host: None,
        smtp_port: 587,
        smtp_username: None,
        smtp_password_configured: true,
        smtp_from_email: None,
        gmail_client_id: Some("client-id".to_owned()),
        gmail_client_id_from_environment: false,
        gmail_client_secret_configured: true,
        gmail_client_secret_from_environment: false,
        gmail_connected: true,
        gmail_from_email: Some("sender@example.com".to_owned()),
        gmail_oauth_redirect_uri: "https://example.com/callback".to_owned(),
    };
    let json = serde_json::to_value(response).expect("response serializes");

    assert!(json.get("smtp_password").is_none());
    assert!(json.get("gmail_client_secret").is_none());
    assert!(json.get("gmail_refresh_token").is_none());
    assert_eq!(json["smtp_password_configured"], true);
    assert_eq!(json["gmail_client_secret_configured"], true);
    assert_eq!(json["gmail_connected"], true);
}
