//! WebTransport TLS configuration.

use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use rcgen::{CertificateParams, KeyPair};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use time::{Duration, OffsetDateTime};
use tracing::info;
use x509_parser::prelude::parse_x509_certificate;

const DEFAULT_DEV_CERT_PATH: &str = "target/cheenhub-dev/webtransport-cert.pem";
const DEFAULT_DEV_KEY_PATH: &str = "target/cheenhub-dev/webtransport-key.pem";
const DEV_CERT_LIFETIME_DAYS: i64 = 13;

/// Resolved WebTransport TLS configuration.
#[derive(Debug)]
pub(crate) struct TlsConfig {
    /// PEM certificate path used by the listener.
    pub(crate) cert_path: String,
    /// PEM private key path used by the listener.
    pub(crate) key_path: String,
}

/// Ensures WebTransport TLS files exist before the backend starts.
pub(crate) fn ensure_tls_config(
    cert_path: Option<&str>,
    key_path: Option<&str>,
) -> anyhow::Result<TlsConfig> {
    let cert_path = cert_path.unwrap_or(DEFAULT_DEV_CERT_PATH);
    let key_path = key_path.unwrap_or(DEFAULT_DEV_KEY_PATH);
    let using_default_paths =
        cert_path == DEFAULT_DEV_CERT_PATH && key_path == DEFAULT_DEV_KEY_PATH;

    if using_default_paths
        && Path::new(cert_path).exists()
        && Path::new(key_path).exists()
        && !is_webtransport_dev_certificate_usable(cert_path)?
    {
        info!(
            cert_path,
            key_path, "regenerating WebTransport dev TLS certificate"
        );
        generate_dev_certificate(cert_path, key_path)?;
    }

    if !Path::new(cert_path).exists() || !Path::new(key_path).exists() {
        if !using_default_paths {
            return Err(anyhow!(
                "WEBTRANSPORT_TLS_CERT_PATH and WEBTRANSPORT_TLS_KEY_PATH must both point to existing files"
            ));
        }
        generate_dev_certificate(cert_path, key_path)?;
    }

    let certificates = load_certificates(cert_path)?;
    load_private_key(key_path)?;
    if let Some(certificate) = certificates.first() {
        info!(
            cert_path,
            key_path,
            cert_sha256 = certificate_sha256_hex(certificate),
            "using WebTransport TLS certificate"
        );
    }

    Ok(TlsConfig {
        cert_path: cert_path.to_owned(),
        key_path: key_path.to_owned(),
    })
}

/// Loads a PEM certificate chain.
pub(crate) fn load_certificates(path: &str) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    let file =
        File::open(path).with_context(|| format!("failed to open certificate PEM {path}"))?;
    let mut reader = BufReader::new(file);
    let certificates = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read certificate PEM {path}"))?;
    if certificates.is_empty() {
        return Err(anyhow!(
            "WEBTRANSPORT_TLS_CERT_PATH must contain at least one certificate"
        ));
    }

    Ok(certificates)
}

/// Loads a PEM private key.
pub(crate) fn load_private_key(path: &str) -> anyhow::Result<PrivateKeyDer<'static>> {
    let file =
        File::open(path).with_context(|| format!("failed to open private key PEM {path}"))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .with_context(|| format!("failed to read private key PEM {path}"))?
        .ok_or_else(|| anyhow!("WEBTRANSPORT_TLS_KEY_PATH must contain a private key"))
}

fn generate_dev_certificate(cert_path: &str, key_path: &str) -> anyhow::Result<()> {
    let cert_path = PathBuf::from(cert_path);
    let key_path = PathBuf::from(key_path);
    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create WebTransport dev certificate directory {}",
                parent.display()
            )
        })?;
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create WebTransport dev key directory {}",
                parent.display()
            )
        })?;
    }

    let key_pair = KeyPair::generate().context("failed to generate WebTransport dev key")?;
    let now = OffsetDateTime::now_utc();
    let mut params = CertificateParams::new(vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ])
    .context("failed to generate WebTransport dev certificate")?;
    params.not_before = now - Duration::minutes(1);
    params.not_after = now + Duration::days(DEV_CERT_LIFETIME_DAYS);

    let certificate = params
        .self_signed(&key_pair)
        .context("failed to sign WebTransport dev certificate")?;
    let certificate_pem = certificate.pem();
    let private_key_pem = key_pair.serialize_pem();

    write_private_file(&cert_path, certificate_pem.as_bytes())
        .with_context(|| format!("failed to write {}", cert_path.display()))?;
    write_private_file(&key_path, private_key_pem.as_bytes())
        .with_context(|| format!("failed to write {}", key_path.display()))?;

    info!(
        cert_path = %cert_path.display(),
        key_path = %key_path.display(),
        "generated WebTransport dev TLS certificate"
    );
    Ok(())
}

fn is_webtransport_dev_certificate_usable(path: &str) -> anyhow::Result<bool> {
    let certificates = load_certificates(path)?;
    let Some(certificate) = certificates.first() else {
        return Ok(false);
    };
    let (_, parsed) = parse_x509_certificate(certificate.as_ref())
        .map_err(|error| anyhow!("failed to parse WebTransport dev certificate: {error}"))?;
    let validity = parsed.validity();
    let lifetime = validity.not_after.to_datetime() - validity.not_before.to_datetime();

    Ok(lifetime <= Duration::days(14))
}

fn write_private_file(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

fn certificate_sha256_hex(certificate: &CertificateDer<'_>) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(certificate.as_ref());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
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
        assert!(
            is_webtransport_dev_certificate_usable(cert_path.to_str().expect("utf-8 cert path"))
                .expect("certificate parses")
        );
        load_private_key(key_path.to_str().expect("utf-8 key path")).expect("private key loads");

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
}
