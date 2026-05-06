//! Client build-time configuration.

use std::fs::File;
use std::io::BufReader;
use std::{env, path::Path};

use sha2::{Digest, Sha256};

const DEFAULT_DEV_CERT_PATH: &str = "../../target/cheenhub-dev/webtransport-cert.pem";

fn main() {
    println!("cargo:rerun-if-changed=../../.env");
    println!("cargo:rerun-if-changed={DEFAULT_DEV_CERT_PATH}");

    dotenvy::from_filename("../../.env").ok();

    forward_env("CHEENHUB_JWT_PUBLIC_KEY_BASE64");
    forward_env("CHEENHUB_REALTIME_URL");
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

fn forward_env(key: &str) -> bool {
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
