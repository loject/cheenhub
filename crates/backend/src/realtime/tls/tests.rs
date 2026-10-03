use super::*;
use uuid::Uuid;

#[test]
fn generates_dev_certificate_files() {
    let directory =
        std::env::temp_dir().join(format!("cheenhub-webtransport-test-{}", Uuid::new_v4()));
    let cert_path = directory.join("cert.pem");
    let key_path = directory.join("key.pem");

    generate_dev_certificate(
        cert_path.to_str().expect("utf-8 cert path"),
        key_path.to_str().expect("utf-8 key path"),
    )
    .expect("dev certificate is generated");

    assert!(cert_path.exists());
    assert!(key_path.exists());
    assert!(
        !load_certificates(cert_path.to_str().expect("utf-8 cert path"))
            .expect("certificates load")
            .is_empty()
    );
    assert_eq!(
        webtransport_dev_certificate_status(cert_path.to_str().expect("utf-8 cert path"))
            .expect("certificate parses"),
        DevCertificateStatus::Usable
    );
    load_private_key(key_path.to_str().expect("utf-8 key path")).expect("private key loads");

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn rejects_expired_dev_certificate_files() {
    let directory =
        std::env::temp_dir().join(format!("cheenhub-webtransport-test-{}", Uuid::new_v4()));
    let cert_path = directory.join("cert.pem");
    let key_path = directory.join("key.pem");
    let now = OffsetDateTime::now_utc();

    generate_dev_certificate_with_validity(
        cert_path.to_str().expect("utf-8 cert path"),
        key_path.to_str().expect("utf-8 key path"),
        now - Duration::days(13),
        now - Duration::minutes(1),
    )
    .expect("expired dev certificate is generated");

    assert_eq!(
        webtransport_dev_certificate_status(cert_path.to_str().expect("utf-8 cert path"))
            .expect("certificate parses"),
        DevCertificateStatus::Expired
    );
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn rejects_missing_custom_tls_pair_without_overwriting() {
    let directory =
        std::env::temp_dir().join(format!("cheenhub-webtransport-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).expect("test directory is created");
    let cert_path = directory.join("cert.pem");
    let key_path = directory.join("key.pem");
    fs::write(&cert_path, b"do not overwrite").expect("test cert is written");

    let error = ensure_tls_config(
        Some(cert_path.to_str().expect("utf-8 cert path")),
        Some(key_path.to_str().expect("utf-8 key path")),
    )
    .expect_err("custom TLS pair must already exist");

    assert!(
        error
            .to_string()
            .contains("must both point to existing files")
    );
    assert_eq!(
        fs::read(&cert_path).expect("test cert is still readable"),
        b"do not overwrite"
    );
    assert!(!key_path.exists());

    let _ = fs::remove_dir_all(directory);
}

#[test]
fn validates_certificate_and_private_key_pair_before_reload() {
    let directory =
        std::env::temp_dir().join(format!("cheenhub-webtransport-test-{}", Uuid::new_v4()));
    let first_cert = directory.join("first-cert.pem");
    let first_key = directory.join("first-key.pem");
    let second_cert = directory.join("second-cert.pem");
    let second_key = directory.join("second-key.pem");

    generate_dev_certificate(
        first_cert.to_str().expect("utf-8 cert path"),
        first_key.to_str().expect("utf-8 key path"),
    )
    .expect("first TLS pair is generated");
    generate_dev_certificate(
        second_cert.to_str().expect("utf-8 cert path"),
        second_key.to_str().expect("utf-8 key path"),
    )
    .expect("second TLS pair is generated");

    build_server_config(
        first_cert.to_str().expect("utf-8 cert path"),
        first_key.to_str().expect("utf-8 key path"),
    )
    .expect("matching pair is accepted");
    assert!(
        build_server_config(
            first_cert.to_str().expect("utf-8 cert path"),
            second_key.to_str().expect("utf-8 key path"),
        )
        .is_err()
    );

    let _ = fs::remove_dir_all(directory);
}
