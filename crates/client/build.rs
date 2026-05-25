//! Client build-time configuration.

use std::fs::{self, File};
use std::io::BufReader;
use std::{
    env,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

const DEFAULT_DEV_CERT_PATH: &str = "../../target/cheenhub-dev/webtransport-cert.pem";
const SW_TEMPLATE_PATH: &str = "public/sw.template.js";
const SW_OUTPUT_PATH: &str = "public/sw.js";

mod file_lines {
    include!("../../build_support/file_lines.rs");
}

fn main() {
    file_lines::check_workspace_file_lines();

    println!("cargo:rerun-if-changed=../../.env");
    println!("cargo:rerun-if-changed={DEFAULT_DEV_CERT_PATH}");
    println!("cargo:rerun-if-changed={SW_TEMPLATE_PATH}");
    println!("cargo:rerun-if-env-changed=CHEENHUB_APP_VERSION");

    dotenvy::from_filename("../../.env").ok();

    let app_version = resolve_app_version();
    println!("cargo:rustc-env=CHEENHUB_APP_VERSION={app_version}");
    generate_service_worker(&app_version);

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

fn resolve_app_version() -> String {
    env::var("CHEENHUB_APP_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("v{}-dev", env!("CARGO_PKG_VERSION")))
}

fn generate_service_worker(app_version: &str) {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let template_path = manifest_dir.join(SW_TEMPLATE_PATH);
    let output_path = manifest_dir.join(SW_OUTPUT_PATH);
    let template = fs::read_to_string(&template_path).expect("service worker template is readable");
    let rendered = template.replace("__CHEENHUB_APP_VERSION__", &js_string_literal(app_version));

    if fs::read_to_string(&output_path).ok().as_deref() == Some(rendered.as_str()) {
        return;
    }

    fs::write(output_path, rendered).expect("service worker can be generated");
}

fn js_string_literal(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| character.escape_default())
        .collect()
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
    let cert_path =
        env::var("WEBTRANSPORT_TLS_CERT_PATH").unwrap_or_else(|_| DEFAULT_DEV_CERT_PATH.to_owned());
    let Some(hash) = certificate_sha256_hex(&cert_path) else {
        println!(
            "cargo:warning=CHEENHUB_REALTIME_CERT_SHA256 is not set and no WebTransport certificate was found at {cert_path}"
        );
        return;
    };

    println!("cargo:rustc-env=CHEENHUB_REALTIME_CERT_SHA256={hash}");
}

fn certificate_sha256_hex(path: &str) -> Option<String> {
    let file = File::open(Path::new(path)).ok()?;
    let mut reader = BufReader::new(file);
    let certificate = rustls_pemfile::certs(&mut reader).next()?.ok()?;
    let digest = Sha256::digest(certificate.as_ref());

    Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}
