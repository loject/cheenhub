//! Client build-time configuration.

use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::{
    env,
    path::{Path, PathBuf},
};

use rcgen::{CertificateParams, KeyPair};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

const DEFAULT_DEV_CERT_PATH: &str = "target/cheenhub-dev/webtransport-cert.pem";
const DEFAULT_DEV_KEY_PATH: &str = "target/cheenhub-dev/webtransport-key.pem";
const DEV_CERT_LIFETIME_DAYS: i64 = 13;

mod file_lines {
    include!("../../build_support/file_lines.rs");
}

fn main() {
    file_lines::check_workspace_file_lines();
    validate_platform_features();
    prepare_installer_payload();

    println!("cargo:rerun-if-changed=../../.env");
    println!("cargo:rerun-if-changed=../../{DEFAULT_DEV_CERT_PATH}");
    println!("cargo:rerun-if-changed=../../{DEFAULT_DEV_KEY_PATH}");
    println!("cargo:rerun-if-env-changed=CHEENHUB_APP_VERSION");

    dotenvy::from_filename("../../.env").ok();

    let app_version = resolve_app_version();
    println!("cargo:rustc-env=CHEENHUB_APP_VERSION={app_version}");

    forward_env("CHEENHUB_JWT_PUBLIC_KEY_BASE64");
    forward_env("CHEENHUB_API_BASE_URL");
    forward_env("CHEENHUB_REALTIME_URL");
    forward_env("CHEENHUB_REALTIME_WS_URL");
    if !forward_env("CHEENHUB_REALTIME_CERT_SHA256") {
        forward_webtransport_cert_hash();
    }

    if env::var("CHEENHUB_JWT_KEY_ID").is_err()
        && let Ok(key_id) = env::var("JWT_KEY_ID")
    {
        println!("cargo:rustc-env=CHEENHUB_JWT_KEY_ID={key_id}");
        return;
    }

    forward_env("CHEENHUB_JWT_KEY_ID");
}

fn prepare_installer_payload() {
    println!("cargo:rerun-if-env-changed=CHEENHUB_INSTALLER_PAYLOAD");
    println!("cargo:rerun-if-env-changed=CHEENHUB_INSTALLER_PAYLOAD_NAME");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set"));
    let generated_path = out_dir.join("installer_payload.rs");
    let Some(payload_path) = env::var_os("CHEENHUB_INSTALLER_PAYLOAD").map(PathBuf::from) else {
        write_installer_payload_module(&generated_path, None, None);
        return;
    };

    println!("cargo:rerun-if-changed={}", payload_path.display());
    let payload_name = env::var("CHEENHUB_INSTALLER_PAYLOAD_NAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            payload_path
                .file_name()
                .and_then(|value| value.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "cheenhub-installer-payload.exe".to_owned());
    let payload_copy = out_dir.join("cheenhub-installer-payload.bin");
    fs::copy(&payload_path, &payload_copy).unwrap_or_else(|error| {
        panic!(
            "failed to copy installer payload {} to {}: {error}",
            payload_path.display(),
            payload_copy.display()
        )
    });

    write_installer_payload_module(&generated_path, Some(&payload_copy), Some(&payload_name));
}

fn write_installer_payload_module(
    generated_path: &Path,
    payload_path: Option<&Path>,
    payload_name: Option<&str>,
) {
    let payload_bytes = payload_path
        .map(|path| format!("Some(include_bytes!(r#\"{}\"#))", path.display()))
        .unwrap_or_else(|| "None".to_owned());
    let payload_name = payload_name
        .map(|name| format!("Some(r#\"{name}\"#)"))
        .unwrap_or_else(|| "None".to_owned());
    let module = format!(
        "pub const INSTALLER_PAYLOAD: Option<&'static [u8]> = {payload_bytes};\n\
         pub const INSTALLER_PAYLOAD_NAME: Option<&'static str> = {payload_name};\n"
    );

    fs::write(generated_path, module).expect("installer payload module can be written");
}

fn validate_platform_features() {
    let enabled_features = ["web", "windows", "linux", "macos", "android"]
        .into_iter()
        .filter(|feature| cargo_feature_enabled(feature))
        .collect::<Vec<_>>();
    if enabled_features.len() > 1 {
        panic!(
            "Выберите ровно одну platform feature для клиента: web, windows, linux или macos. Сейчас включены: {}.",
            enabled_features.join(", ")
        );
    }

    let desktop_enabled = cargo_feature_enabled("desktop");
    if desktop_enabled {
        let enabled_desktop_platforms = ["windows", "linux", "macos"]
            .into_iter()
            .filter(|feature| cargo_feature_enabled(feature))
            .collect::<Vec<_>>();
        if enabled_desktop_platforms.len() != 1 {
            panic!(
                "Desktop-сборка клиента должна явно выбрать ровно одну platform feature: windows, linux или macos. Сейчас включены: {}.",
                if enabled_desktop_platforms.is_empty() {
                    "нет".to_owned()
                } else {
                    enabled_desktop_platforms.join(", ")
                }
            );
        }
    }

    if cargo_feature_enabled("mobile") && !cargo_feature_enabled("android") {
        panic!("Mobile client build must explicitly enable the android platform feature.");
    }
}

fn cargo_feature_enabled(feature: &str) -> bool {
    let env_key = format!("CARGO_FEATURE_{}", feature.replace('-', "_").to_uppercase());
    env::var_os(env_key).is_some()
}

fn resolve_app_version() -> String {
    env::var("CHEENHUB_APP_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("v{}-dev", env!("CARGO_PKG_VERSION")))
}

fn forward_env(key: &str) -> bool {
    println!("cargo:rerun-if-env-changed={key}");
    if let Ok(value) = env::var(key) {
        println!("cargo:rustc-env={key}={value}");
        true
    } else {
        false
    }
}

fn forward_webtransport_cert_hash() {
    let cert_path_value = env_path_value("WEBTRANSPORT_TLS_CERT_PATH", DEFAULT_DEV_CERT_PATH);
    let key_path_value = env_path_value("WEBTRANSPORT_TLS_KEY_PATH", DEFAULT_DEV_KEY_PATH);
    let cert_path = workspace_path(&cert_path_value);
    let key_path = workspace_path(&key_path_value);
    if uses_default_dev_tls_paths(&cert_path_value, &key_path_value)
        && (!cert_path.exists() || !key_path.exists())
    {
        generate_dev_certificate(&cert_path, &key_path);
    }

    let Some(hash) = certificate_sha256_hex(&cert_path) else {
        println!(
            "cargo:warning=CHEENHUB_REALTIME_CERT_SHA256 is not set and no WebTransport certificate was found at {}",
            cert_path.display()
        );
        return;
    };

    println!("cargo:rustc-env=CHEENHUB_REALTIME_CERT_SHA256={hash}");
}

fn env_path_value(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn workspace_path(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return path;
    }

    let workspace_root =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir is set")).join("../..");
    workspace_root.join(path)
}

fn uses_default_dev_tls_paths(cert_path: &str, key_path: &str) -> bool {
    normalize_path_value(cert_path) == DEFAULT_DEV_CERT_PATH
        && normalize_path_value(key_path) == DEFAULT_DEV_KEY_PATH
}

fn normalize_path_value(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_owned()
}

fn generate_dev_certificate(cert_path: &Path, key_path: &Path) {
    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent).expect("WebTransport dev certificate directory can be created");
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).expect("WebTransport dev key directory can be created");
    }

    let key_pair = KeyPair::generate().expect("WebTransport dev key can be generated");
    let now = OffsetDateTime::now_utc();
    let mut params = CertificateParams::new(vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ])
    .expect("WebTransport dev certificate params can be generated");
    params.not_before = now - Duration::minutes(1);
    params.not_after = now + Duration::days(DEV_CERT_LIFETIME_DAYS);

    let certificate = params
        .self_signed(&key_pair)
        .expect("WebTransport dev certificate can be signed");
    write_private_file(cert_path, certificate.pem().as_bytes());
    write_private_file(key_path, key_pair.serialize_pem().as_bytes());
}

fn write_private_file(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).expect("private file can be created");
    file.write_all(bytes).expect("private file can be written");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = file
            .metadata()
            .expect("private file metadata is readable")
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions).expect("private file permissions can be set");
    }
}

fn certificate_sha256_hex(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let certificate = rustls_pemfile::certs(&mut reader).next()?.ok()?;
    let digest = Sha256::digest(certificate.as_ref());

    Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}
