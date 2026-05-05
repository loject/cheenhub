//! Client build-time configuration.

use std::env;

fn main() {
    println!("cargo:rerun-if-changed=../../.env");

    dotenvy::from_filename("../../.env").ok();

    forward_env("CHEENHUB_JWT_PUBLIC_KEY_BASE64");

    if env::var("CHEENHUB_JWT_KEY_ID").is_err()
        && let Ok(key_id) = env::var("JWT_KEY_ID")
    {
        println!("cargo:rustc-env=CHEENHUB_JWT_KEY_ID={key_id}");
        return;
    }

    forward_env("CHEENHUB_JWT_KEY_ID");
}

fn forward_env(key: &str) {
    if let Ok(value) = env::var(key) {
        println!("cargo:rustc-env={key}={value}");
    }
}
